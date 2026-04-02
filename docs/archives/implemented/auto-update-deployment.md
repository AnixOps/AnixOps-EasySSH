# Auto-Update System Deployment

## Infrastructure Overview

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Client App    │────▶│   CloudFront    │────▶│   Origin (S3)   │
│  (Win/Mac/Lin)  │     │    CDN          │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
         │                                               │
         │                                               │
         │                       ┌─────────────────┐      │
         └──────────────────────▶│  Update Server  │◀─────┘
                                 │   (API Server)  │
                                 └─────────────────┘
                                         │
                                 ┌───────┴───────┐
                                 │               │
                        ┌────────▼────────┐ ┌────▼────────────┐
                        │   PostgreSQL    │ │   Redis Cache   │
                        │    (Stats)      │ │   (Rollouts)    │
                        └─────────────────┘ └─────────────────┘
```

## AWS CloudFormation Template

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: EasySSH Update Server Infrastructure

Parameters:
  Environment:
    Type: String
    Default: production
    AllowedValues: [staging, production]

Resources:
  # S3 Bucket for update packages
  UpdatePackagesBucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName: !Sub easyssh-updates-${Environment}
      PublicAccessBlockConfiguration:
        BlockPublicAcls: false
        BlockPublicPolicy: false
        IgnorePublicAcls: false
        RestrictPublicBuckets: false
      CorsConfiguration:
        CorsRules:
          - AllowedHeaders: ['*']
            AllowedMethods: [GET, HEAD]
            AllowedOrigins: ['*']
            MaxAge: 3600

  # CloudFront Distribution
  UpdateCDN:
    Type: AWS::CloudFront::Distribution
    Properties:
      DistributionConfig:
        Origins:
          - DomainName: !GetAtt UpdatePackagesBucket.RegionalDomainName
            Id: S3Origin
            S3OriginConfig:
              OriginAccessIdentity: ''
          - DomainName: !GetAtt UpdateApiServer.Endpoint
            Id: APIOrigin
            CustomOriginConfig:
              OriginProtocolPolicy: https-only
        Enabled: true
        DefaultCacheBehavior:
          TargetOriginId: S3Origin
          ViewerProtocolPolicy: https-only
          AllowedMethods: [GET, HEAD, OPTIONS]
          CachedMethods: [GET, HEAD]
          Compress: true
          ForwardedValues:
            QueryString: true
            Headers:
              - Origin
              - Access-Control-Request-Headers
              - Access-Control-Request-Method
          TTL: 86400
          MaxTTL: 31536000
        CacheBehaviors:
          - PathPattern: /api/*
            TargetOriginId: APIOrigin
            ViewerProtocolPolicy: https-only
            AllowedMethods: [DELETE, GET, HEAD, OPTIONS, PATCH, POST, PUT]
            ForwardedValues:
              QueryString: true
              Headers: ['*']
            TTL: 0
            MaxTTL: 0
            DefaultTTL: 0
        PriceClass: PriceClass_All
        ViewerCertificate:
          CloudFrontDefaultCertificate: true

  # ECS Cluster for Update API Server
  UpdateApiCluster:
    Type: AWS::ECS::Cluster
    Properties:
      ClusterName: !Sub easyssh-update-api-${Environment}
      CapacityProviders:
        - FARGATE
      DefaultCapacityProviderStrategy:
        - CapacityProvider: FARGATE
          Weight: 1

  # ECS Task Definition
  UpdateApiTask:
    Type: AWS::ECS::TaskDefinition
    Properties:
      Family: update-api
      NetworkMode: awsvpc
      RequiresCompatibilities:
        - FARGATE
      Cpu: 256
      Memory: 512
      ContainerDefinitions:
        - Name: update-api
          Image: !Sub ${AWS::AccountId}.dkr.ecr.${AWS::Region}.amazonaws.com/easyssh/update-api:latest
          PortMappings:
            - ContainerPort: 8080
          Environment:
            - Name: RUST_LOG
              Value: info
            - Name: DATABASE_URL
              Value: !Sub postgresql://user:pass@${Database.Endpoint}/easyssh_updates
            - Name: REDIS_URL
              Value: !Sub redis://${RedisCache.RedisEndpointAddress}:6379
            - Name: CDN_BASE_URL
              Value: !Sub https://${UpdateCDN.DomainName}
          LogConfiguration:
            LogDriver: awslogs
            Options:
              awslogs-group: !Ref UpdateApiLogGroup
              awslogs-region: !Ref AWS::Region
              awslogs-stream-prefix: update-api

  # RDS PostgreSQL for analytics
  Database:
    Type: AWS::RDS::DBInstance
    Properties:
      DBInstanceIdentifier: !Sub easyssh-updates-${Environment}
      DBInstanceClass: db.t3.micro
      Engine: postgres
      EngineVersion: '14.5'
      AllocatedStorage: 20
      StorageType: gp2
      MasterUsername: easyssh
      MasterUserPassword: !Sub '{{resolve:secretsmanager:easyssh/db/password:SecretString:password}}'
      VPCSecurityGroups:
        - !Ref DatabaseSecurityGroup
      PubliclyAccessible: false

  # ElastiCache Redis for rollout management
  RedisCache:
    Type: AWS::ElastiCache::CacheCluster
    Properties:
      CacheNodeType: cache.t3.micro
      Engine: redis
      NumCacheNodes: 1
      VpcSecurityGroupIds:
        - !Ref RedisSecurityGroup

  # Security Groups
  DatabaseSecurityGroup:
    Type: AWS::EC2::SecurityGroup
    Properties:
      GroupDescription: Database security group
      SecurityGroupIngress:
        - IpProtocol: tcp
          FromPort: 5432
          ToPort: 5432
          SourceSecurityGroupId: !Ref ApiServiceSecurityGroup

  RedisSecurityGroup:
    Type: AWS::EC2::SecurityGroup
    Properties:
      GroupDescription: Redis security group
      SecurityGroupIngress:
        - IpProtocol: tcp
          FromPort: 6379
          ToPort: 6379
          SourceSecurityGroupId: !Ref ApiServiceSecurityGroup

  ApiServiceSecurityGroup:
    Type: AWS::EC2::SecurityGroup
    Properties:
      GroupDescription: API service security group
      SecurityGroupIngress:
        - IpProtocol: tcp
          FromPort: 8080
          ToPort: 8080
          CidrIp: 0.0.0.0/0

  # CloudWatch Log Group
  UpdateApiLogGroup:
    Type: AWS::Logs::LogGroup
    Properties:
      LogGroupName: /ecs/easyssh/update-api
      RetentionInDays: 30

Outputs:
  CDNDomain:
    Description: CloudFront Domain Name
    Value: !GetAtt UpdateCDN.DomainName
  DatabaseEndpoint:
    Description: RDS Endpoint
    Value: !GetAtt Database.Endpoint.Address
  RedisEndpoint:
    Description: Redis Endpoint
    Value: !GetAtt RedisCache.RedisEndpointAddress
```

## Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-update-api
  labels:
    app: easyssh-update-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: easyssh-update-api
  template:
    metadata:
      labels:
        app: easyssh-update-api
    spec:
      containers:
      - name: api
        image: easyssh/update-api:latest
        ports:
        - containerPort: 8080
        env:
        - name: RUST_LOG
          value: "info"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: update-api-secrets
              key: database-url
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: update-api-secrets
              key: redis-url
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "256Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: easyssh-update-api
spec:
  selector:
    app: easyssh-update-api
  ports:
  - port: 80
    targetPort: 8080
  type: ClusterIP
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: easyssh-update-api
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
  tls:
  - hosts:
    - updates.easyssh.dev
    secretName: easyssh-update-api-tls
  rules:
  - host: updates.easyssh.dev
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: easyssh-update-api
            port:
              number: 80
```

## Docker Compose (Development)

```yaml
version: '3.8'

services:
  update-api:
    build:
      context: ./pro-server
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=debug
      - DATABASE_URL=postgresql://postgres:postgres@postgres:5432/easyssh_updates
      - REDIS_URL=redis://redis:6379
      - CDN_BASE_URL=http://localhost:9000
    depends_on:
      - postgres
      - redis
      - minio

  postgres:
    image: postgres:14-alpine
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: easyssh_updates
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

  minio:
    image: minio/minio:latest
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    volumes:
      - minio_data:/data
    ports:
      - "9000:9000"
      - "9001:9001"

  create-buckets:
    image: minio/mc:latest
    depends_on:
      - minio
    entrypoint: >
      /bin/sh -c "
      sleep 5 &&
      mc alias set local http://minio:9000 minioadmin minioadmin &&
      mc mb local/easyssh-updates || true &&
      mc anonymous set download local/easyssh-updates || true
      "

volumes:
  postgres_data:
  minio_data:
```

## Release Management Script

```bash
#!/bin/bash
# release.sh - Automated release script

set -e

VERSION=$1
BUILD_NUMBER=$2
CHANNEL=${3:-stable}

if [ -z "$VERSION" ] || [ -z "$BUILD_NUMBER" ]; then
    echo "Usage: $0 <version> <build_number> [channel]"
    exit 1
fi

echo "Creating release $VERSION (build $BUILD_NUMBER) for channel $CHANNEL"

# Build all platforms
echo "Building Windows..."
cargo build --release --target x86_64-pc-windows-msvc
# Create MSI using WiX or NSIS

# Upload to S3
echo "Uploading packages..."
aws s3 cp target/release/EasySSH-$VERSION-x86_64.msi \
    s3://easyssh-updates/releases/$VERSION/EasySSH-$VERSION-x86_64.msi

aws s3 cp target/release/EasySSH-$VERSION-x86_64.msi.sig \
    s3://easyssh-updates/releases/$VERSION/EasySSH-$VERSION-x86_64.msi.sig

# Create delta patches
echo "Creating delta patches..."
for old_version in $(get_last_5_versions); do
    bsdiff \
        s3://easyssh-updates/releases/$old_version/EasySSH-$old_version-x86_64.msi \
        s3://easyssh-updates/releases/$VERSION/EasySSH-$VERSION-x86_64.msi \
        s3://easyssh-updates/delta/$old_version-$VERSION.patch
done

# Register with update server
echo "Registering release..."
curl -X POST https://updates.easyssh.dev/api/v1/admin/releases \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d @- <<EOF
{
    "version": "$VERSION",
    "build_number": $BUILD_NUMBER,
    "channel": "$CHANNEL",
    "platforms": {
        "windows": {
            "size": $(stat -c%s "target/release/EasySSH-$VERSION-x86_64.msi"),
            "sha256": "$(sha256sum target/release/EasySSH-$VERSION-x86_64.msi | cut -d' ' -f1)"
        }
    },
    "rollout_percentage": 5
}
EOF

echo "Release $VERSION created successfully!"
```

## Rollout Monitoring

```python
#!/usr/bin/env python3
# monitor_rollout.py

import requests
import sys
from datetime import datetime, timedelta

def monitor_rollout(version):
    """Monitor rollout metrics for a version"""

    base_url = "https://updates.easyssh.dev/api/v1"

    # Get update stats
    stats = requests.get(f"{base_url}/admin/stats/{version}").json()

    print(f"Rollout Status for {version}")
    print("=" * 50)
    print(f"Rollout Percentage: {stats['rollout_percentage']}%")
    print(f"Downloads: {stats['downloads']}")
    print(f"Successful Installs: {stats['successful_installs']}")
    print(f"Failed Installs: {stats['failed_installs']}")
    print(f"Error Rate: {stats['error_rate']:.2f}%")
    print(f"Rollback Rate: {stats['rollback_rate']:.2f}%")

    # Error breakdown
    if stats['errors']:
        print("\nError Breakdown:")
        for error, count in stats['errors'].items():
            print(f"  {error}: {count}")

    # Recommend action
    if stats['error_rate'] > 5.0:
        print("\n⚠️  WARNING: Error rate exceeds 5%! Consider halting rollout.")
        return False
    elif stats['error_rate'] > 1.0:
        print("\n⚠️  CAUTION: Error rate is elevated. Monitor closely.")
        return True
    else:
        print("\n✅ Rollout is healthy. Safe to increase rollout percentage.")
        return True

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: monitor_rollout.py <version>")
        sys.exit(1)

    version = sys.argv[1]
    healthy = monitor_rollout(version)
    sys.exit(0 if healthy else 1)
```
