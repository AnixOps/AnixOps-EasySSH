# AWS Deployment Guide for EasySSH

> Complete deployment guide for hosting EasySSH on Amazon Web Services

---

## Overview

This guide covers multiple deployment options for EasySSH on AWS:

| Option | Use Case | Complexity | Cost |
|--------|----------|------------|------|
| **AWS ECS (Fargate)** | Production, auto-scaling | Medium | $$$ |
| **AWS EKS (Kubernetes)** | Enterprise, multi-cluster | High | $$$$ |
| **EC2 Standalone** | Simple, development | Low | $$ |
| **AWS App Runner** | Quick deploy, managed | Low | $$$ |

---

## Prerequisites

### Required Tools

```bash
# AWS CLI
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip && sudo ./aws/install

# Configure AWS credentials
aws configure

# kubectl (for EKS)
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
chmod +x kubectl && sudo mv kubectl /usr/local/bin/

# Docker
curl -fsSL https://get.docker.com | sh

# Helm (for EKS)
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
```

### AWS Requirements

- AWS Account with appropriate permissions
- VPC with public/private subnets
- IAM user/role with ECR, ECS/EKS, EC2 permissions
- Route 53 hosted zone (optional, for custom domain)

---

## Option 1: AWS ECS (Fargate)

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         AWS Cloud                            │
│  ┌────────────────────────────────────────────────────────┐ │
│  │                        VPC                              │ │
│  │  ┌──────────────┐     ┌──────────────────────────────┐ │ │
│  │  │  ALB         │     │    Private Subnets           │ │ │
│  │  │  (Public)    │────▶│  ┌────────┐  ┌────────┐     │ │ │
│  │  │  :443       │     │  │ECS Task│  │ECS Task│     │ │ │
│  │  └──────────────┘     │  │ :8080  │  │ :8080  │     │ │ │
│  │                       │  └────────┘  └────────┘     │ │ │
│  │                       │       │            │         │ │ │
│  │                       │       ▼            ▼         │ │ │
│  │                       │  ┌────────────────────────┐  │ │ │
│  │                       │  │   ElastiCache Redis   │  │ │ │
│  │                       │  │   (Session Storage)   │  │ │ │
│  │                       │  └────────────────────────┘  │ │ │
│  │                       └──────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  ECR Repository: easyssh-server                        │ │
│  │  S3 Bucket: easyssh-backups                            │ │
│  │  Secrets Manager: DB credentials                       │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Step 1: Create ECR Repository

```bash
# Create ECR repository
aws ecr create-repository \
  --repository-name easyssh-server \
  --image-scanning-configuration scanOnPush=true

# Login to ECR
aws ecr get-login-password --region us-east-1 | \
  docker login --username AWS --password-stdin \
  $(aws sts get-caller-identity --query Account --output text).dkr.ecr.us-east-1.amazonaws.com
```

### Step 2: Build and Push Image

```dockerfile
# Dockerfile
FROM rust:1.89 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p easyssh-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/easyssh-server /usr/local/bin/
EXPOSE 8080
CMD ["easyssh-server"]
```

```bash
# Build and push
docker build -t easyssh-server .
docker tag easyssh-server:latest \
  $(aws sts get-caller-identity --query Account --output text).dkr.ecr.us-east-1.amazonaws.com/easyssh-server:latest
docker push $(aws sts get-caller-identity --query Account --output text).dkr.ecr.us-east-1.amazonaws.com/easyssh-server:latest
```

### Step 3: Create ECS Cluster

```bash
# Create cluster
aws ecs create-cluster --cluster-name easyssh-cluster

# Create task definition
cat > task-definition.json << 'EOF'
{
  "family": "easyssh-task",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "1024",
  "memory": "2048",
  "containerDefinitions": [{
    "name": "easyssh-server",
    "image": "${ACCOUNT_ID}.dkr.ecr.us-east-1.amazonaws.com/easyssh-server:latest",
    "essential": true,
    "portMappings": [{
      "containerPort": 8080,
      "protocol": "tcp"
    }],
    "environment": [
      {"name": "RUST_LOG", "value": "info"}
    ],
    "secrets": [
      {
        "name": "DATABASE_URL",
        "valueFrom": "arn:aws:secretsmanager:us-east-1:${ACCOUNT_ID}:secret:easyssh/db-url"
      }
    ],
    "logConfiguration": {
      "logDriver": "awslogs",
      "options": {
        "awslogs-group": "/ecs/easyssh",
        "awslogs-region": "us-east-1",
        "awslogs-stream-prefix": "ecs"
      }
    }
  }]
}
EOF

aws ecs register-task-definition --cli-input-json file://task-definition.json
```

### Step 4: Create Service with Load Balancer

```bash
# Create ALB
aws elbv2 create-load-balancer \
  --name easyssh-alb \
  --subnets subnet-xxx subnet-yyy \
  --security-groups sg-xxx

# Create target group
aws elbv2 create-target-group \
  --name easyssh-targets \
  --port 8080 \
  --protocol HTTP \
  --vpc-id vpc-xxx \
  --target-type ip

# Create ECS service
aws ecs create-service \
  --cluster easyssh-cluster \
  --service-name easyssh-service \
  --task-definition easyssh-task:1 \
  --desired-count 2 \
  --launch-type FARGATE \
  --network-configuration "awsvpcConfiguration={subnets=[subnet-xxx],securityGroups=[sg-xxx],assignPublicIp=ENABLED}" \
  --load-balancers "targetGroupArn=arn:aws:elasticloadbalancing:us-east-1:${ACCOUNT_ID}:targetgroup/easyssh-targets/xxx,containerName=easyssh-server,containerPort=8080"
```

---

## Option 2: AWS EKS (Kubernetes)

### Step 1: Create EKS Cluster

```bash
# Create EKS cluster (using eksctl)
cat > cluster-config.yaml << 'EOF'
apiVersion: eksctl.io/v1alpha5
kind: ClusterConfig

metadata:
  name: easyssh-cluster
  region: us-east-1

nodeGroups:
  - name: workers
    instanceType: t3.medium
    desiredCapacity: 3
    minSize: 2
    maxSize: 5
    iam:
      withAddonPolicies:
        albIngress: true
        ebs: true
        efs: true

managedNodeGroups:
  - name: managed-workers
    instanceType: t3.medium
    desiredCapacity: 2
EOF

eksctl create cluster -f cluster-config.yaml
```

### Step 2: Deploy EasySSH

```yaml
# kubernetes/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-server
  labels:
    app: easyssh
spec:
  replicas: 3
  selector:
    matchLabels:
      app: easyssh
  template:
    metadata:
      labels:
        app: easyssh
    spec:
      containers:
      - name: easyssh-server
        image: ${ACCOUNT_ID}.dkr.ecr.us-east-1.amazonaws.com/easyssh-server:latest
        ports:
        - containerPort: 8080
        resources:
          requests:
            cpu: "250m"
            memory: "512Mi"
          limits:
            cpu: "500m"
            memory: "1Gi"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
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
  name: easyssh-service
spec:
  type: LoadBalancer
  selector:
    app: easyssh
  ports:
  - port: 80
    targetPort: 8080
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: easyssh-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: easyssh-server
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

```bash
# Deploy
kubectl apply -f kubernetes/deployment.yaml
```

---

## Option 3: EC2 Standalone

### Quick Deploy with User Data

```bash
# Launch EC2 instance
aws ec2 run-instances \
  --image-id ami-0c55b159cbfafe1f0 \
  --count 1 \
  --instance-type t3.medium \
  --key-name your-key \
  --security-group-ids sg-xxx \
  --subnet-id subnet-xxx \
  --user-data '#!/bin/bash
    # Install Docker
    curl -fsSL https://get.docker.com | sh
    usermod -aG docker ec2-user

    # Pull and run EasySSH
    docker run -d \
      --name easyssh \
      --restart always \
      -p 8080:8080 \
      -v /data/easyssh:/data \
      -e RUST_LOG=info \
      ${ACCOUNT_ID}.dkr.ecr.us-east-1.amazonaws.com/easyssh-server:latest
  '
```

---

## Infrastructure as Code

### Terraform Example

```hcl
# main.tf
provider "aws" {
  region = var.region
}

# VPC
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "5.0.0"

  name = "easyssh-vpc"
  cidr = "10.0.0.0/16"

  azs             = ["${var.region}a", "${var.region}b"]
  private_subnets = ["10.0.1.0/24", "10.0.2.0/24"]
  public_subnets  = ["10.0.101.0/24", "10.0.102.0/24"]

  enable_nat_gateway = true
  single_nat_gateway = true
}

# ECR
resource "aws_ecr_repository" "easyssh" {
  name                 = "easyssh-server"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

# ECS Cluster
resource "aws_ecs_cluster" "easyssh" {
  name = "easyssh-cluster"

  setting {
    name  = "containerInsights"
    value = "enabled"
  }
}

# ALB
resource "aws_lb" "easyssh" {
  name               = "easyssh-alb"
  internal           = false
  load_balancer_type = "application"
  subnets            = module.vpc.public_subnets
  security_groups    = [aws_security_group.alb.id]
}

# Outputs
output "alb_dns_name" {
  value = aws_lb.easyssh.dns_name
}
```

---

## Security Configuration

### IAM Policy

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "ecr:GetAuthorizationToken",
        "ecr:BatchCheckLayerAvailability",
        "ecr:GetDownloadUrlForLayer",
        "ecr:BatchGetImage"
      ],
      "Resource": "*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "secretsmanager:GetSecretValue"
      ],
      "Resource": "arn:aws:secretsmanager:*:*:secret:easyssh/*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "s3:GetObject",
        "s3:PutObject"
      ],
      "Resource": "arn:aws:s3:::easyssh-backups/*"
    }
  ]
}
```

### Security Groups

```bash
# ALB Security Group
aws ec2 create-security-group \
  --group-name easyssh-alb-sg \
  --description "ALB Security Group for EasySSH"

aws ec2 authorize-security-group-ingress \
  --group-id sg-alb \
  --protocol tcp \
  --port 443 \
  --cidr 0.0.0.0/0

# ECS Task Security Group
aws ec2 create-security-group \
  --group-name easyssh-ecs-sg \
  --description "ECS Task Security Group for EasySSH"

aws ec2 authorize-security-group-ingress \
  --group-id sg-ecs \
  --protocol tcp \
  --port 8080 \
  --source-group sg-alb
```

### Secrets Manager

```bash
# Store database credentials
aws secretsmanager create-secret \
  --name easyssh/db-url \
  --secret-string "postgresql://user:password@host:5432/easyssh"

# Store encryption key
aws secretsmanager create-secret \
  --name easyssh/encryption-key \
  --secret-string "$(openssl rand -base64 32)"
```

---

## Monitoring

### CloudWatch Dashboard

```bash
# Create log group
aws logs create-log-group --log-group-name /ecs/easyssh

# Create dashboard
aws cloudwatch put-dashboard \
  --dashboard-name EasySSH \
  --dashboard-body '{
    "widgets": [
      {
        "type": "metric",
        "properties": {
          "metrics": [
            ["AWS/ECS", "CPUUtilization", {"stat": "Average"}],
            ["AWS/ECS", "MemoryUtilization", {"stat": "Average"}]
          ],
          "period": 300,
          "region": "us-east-1"
        }
      }
    ]
  }'
```

### Alerts

```bash
# Create SNS topic for alerts
aws sns create-topic --name easyssh-alerts

# Create CloudWatch alarm
aws cloudwatch put-metric-alarm \
  --alarm-name easyssh-high-cpu \
  --metric-name CPUUtilization \
  --namespace AWS/ECS \
  --statistic Average \
  --period 300 \
  --threshold 80 \
  --comparison-operator GreaterThanThreshold \
  --dimensions Name=ServiceName,Value=easyssh-service \
  --evaluation-periods 2 \
  --alarm-actions arn:aws:sns:us-east-1:${ACCOUNT_ID}:easyssh-alerts
```

---

## Backup to S3

```bash
# Create S3 bucket
aws s3 mb s3://easyssh-backups

# Configure lifecycle
aws s3api put-bucket-lifecycle-configuration \
  --bucket easyssh-backups \
  --lifecycle-configuration '{
    "Rules": [
      {
        "ID": "BackupRetention",
        "Status": "Enabled",
        "Filter": {},
        "Transitions": [
          {"Days": 30, "StorageClass": "STANDARD_IA"},
          {"Days": 90, "StorageClass": "GLACIER"}
        ],
        "Expiration": {"Days": 365}
      }
    ]
  }'
```

---

## Cost Estimation

### Production (ECS Fargate)

| Resource | Configuration | Monthly Cost |
|----------|---------------|--------------|
| ECS Tasks (2x) | 1 vCPU, 2GB RAM | ~$70 |
| ALB | Application Load Balancer | ~$25 |
| NAT Gateway | 1x | ~$35 |
| ECR | 5GB storage | ~$5 |
| S3 | 50GB backups | ~$1 |
| Secrets Manager | 3 secrets | ~$2 |
| CloudWatch | Logs + Metrics | ~$10 |
| **Total** | | **~$148/month** |

### Development (EC2)

| Resource | Configuration | Monthly Cost |
|----------|---------------|--------------|
| EC2 t3.medium | 1 instance | ~$35 |
| EBS | 50GB gp3 | ~$5 |
| **Total** | | **~$40/month** |

---

## Troubleshooting

### Common Issues

```bash
# Check ECS task status
aws ecs describe-tasks \
  --cluster easyssh-cluster \
  --tasks TASK_ID

# View logs
aws logs get-log-events \
  --log-group-name /ecs/easyssh \
  --log-stream-name ecs/easyssh-server/TASK_ID

# Check ALB target health
aws elbv2 describe-target-health \
  --target-group-arn TARGET_GROUP_ARN
```

### Useful Commands

```bash
# Scale ECS service
aws ecs update-service \
  --cluster easyssh-cluster \
  --service easyssh-service \
  --desired-count 5

# Force new deployment
aws ecs update-service \
  --cluster easyssh-cluster \
  --service easyssh-service \
  --force-new-deployment

# SSH into EC2 instance
ssh -i your-key.pem ec2-user@PUBLIC_IP
docker logs easyssh
```

---

## References

- [AWS ECS Documentation](https://docs.aws.amazon.com/ecs/)
- [AWS EKS Documentation](https://docs.aws.amazon.com/eks/)
- [AWS CLI Reference](https://docs.aws.amazon.com/cli/)
- [Terraform AWS Provider](https://registry.terraform.io/providers/hashicorp/aws/)