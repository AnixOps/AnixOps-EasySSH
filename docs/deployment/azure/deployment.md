# EasySSH Azure Deployment Guide

> Complete deployment documentation for EasySSH Pro Server on Microsoft Azure
> Version: 1.0 | Date: 2026-04-03

---

## Table of Contents

1. [Overview](#1-overview)
2. [Prerequisites](#2-prerequisites)
3. [Deployment Options](#3-deployment-options)
4. [Infrastructure as Code](#4-infrastructure-as-code)
5. [Configuration](#5-configuration)
6. [Security](#6-security)
7. [Monitoring](#7-monitoring)
8. [Cost Estimation](#8-cost-estimation)

---

## 1. Overview

### 1.1 Azure Deployment Options

EasySSH Pro Server supports multiple deployment options on Azure:

| Option | Best For | Complexity | Scalability | Cost |
|--------|----------|------------|-------------|------|
| **Azure Kubernetes Service (AKS)** | Production, Enterprise | High | Excellent | Medium-High |
| **Azure Container Instances (ACI)** | Testing, Prototyping | Low | Limited | Low |
| **Azure VM Deployment** | Traditional, BYOD | Medium | Manual | Medium |
| **Azure Container Apps** | Modern Cloud-Native | Medium | Good | Medium |

### 1.2 Architecture Overview

```
+-------------------------------------------------------------------------+
|                     EasySSH Azure Deployment                            |
+-------------------------------------------------------------------------+
|                                                                         |
|   +---------------------------+     +---------------------------+      |
|   |     Azure Front Door     |     |    Azure CDN (Static)     |      |
|   |   (Load Balancer/WAF)    |     |   (Client Downloads)      |      |
|   +------------+--------------+     +---------------------------+      |
|                |                                                        |
|                v                                                        |
|   +---------------------------+                                         |
|   |   Azure Application GW    |                                        |
|   |   (Ingress Controller)    |                                        |
|   +------------+--------------+                                         |
|                |                                                        |
|                v                                                        |
|   +---------------------------+                                         |
|   |      AKS Cluster          |                                        |
|   |  +---------------------+  |                                        |
|   |  |  EasySSH API Pods   |  |                                        |
|   |  |  (3+ replicas)      |  |                                        |
|   |  +---------------------+  |                                        |
|   |  +---------------------+  |                                        |
|   |  |  WebSocket Server   |  |                                        |
|   |  +---------------------+  |                                        |
|   +------------+--------------+                                         |
|                |                                                        |
|                v                                                        |
|   +---------------------------+                                         |
|   |      Data Services        |                                        |
|   |  +---------------------+  |                                        |
|   |  | Azure PostgreSQL    |  |                                        |
|   |  +---------------------+  |                                        |
|   |  +---------------------+  |                                        |
|   |  | Azure Redis Cache   |  |                                        |
|   |  +---------------------+  |                                        |
|   |  +---------------------+  |                                        |
|   |  | Azure Blob Storage  |  |                                        |
|   |  +---------------------+  |                                        |
|   +---------------------------+                                         |
|                                                                         |
|   +---------------------------+     +---------------------------+      |
|   |    Azure Key Vault        |     |    Azure Monitor          |      |
|   |   (Secrets Management)    |     |   (Logs & Metrics)        |      |
|   +---------------------------+     +---------------------------+      |
|                                                                         |
+-------------------------------------------------------------------------+
```

---

## 2. Prerequisites

### 2.1 Azure Account Requirements

- Azure subscription with sufficient credits
- Contributor or Owner role on the subscription
- Azure AD tenant for authentication

### 2.2 Required Tools

```bash
# Install Azure CLI
# Windows (PowerShell)
winget install Microsoft.AzureCLI

# macOS
brew install azure-cli

# Linux (Ubuntu/Debian)
curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash

# Verify installation
az --version
```

### 2.3 Additional Tools

```bash
# Install Kubernetes CLI (kubectl)
az aks install-cli

# Install Helm (for AKS deployments)
# Windows
winget install Helm.Helm

# macOS
brew install helm

# Linux
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash

# Install Bicep CLI
az bicep install

# Install Terraform (optional)
# macOS
brew install terraform

# Windows
winget install HashiCorp.Terraform
```

### 2.4 Initial Azure Setup

```bash
# Login to Azure
az login

# Set default subscription
az account set --subscription "<subscription-id>"

# Verify subscription
az account show

# Create resource group
az group create \
  --name easyssh-prod-rg \
  --location eastus

# Register required providers
az provider register --namespace Microsoft.ContainerService
az provider register --namespace Microsoft.KeyVault
az provider register --namespace Microsoft.DBforPostgreSQL
az provider register --namespace Microsoft.Cache
az provider register --namespace Microsoft.ContainerRegistry
```

---

## 3. Deployment Options

### 3.1 Azure AKS (Kubernetes) Deployment

**Recommended for production workloads.**

#### Create AKS Cluster

```bash
# Create AKS cluster with system node pool
az aks create \
  --resource-group easyssh-prod-rg \
  --name easyssh-aks \
  --node-count 3 \
  --node-vm-size Standard_D2s_v3 \
  --enable-managed-identity \
  --enable-addons monitoring \
  --workspace-resource-id "/subscriptions/<sub-id>/resourcegroups/easyssh-prod-rg/providers/microsoft.operationalinsights/workspaces/easyssh-logs" \
  --generate-ssh-keys \
  --kubernetes-version 1.28 \
  --network-plugin azure \
  --network-policy calico \
  --zones 1 2 3

# Get cluster credentials
az aks get-credentials \
  --resource-group easyssh-prod-rg \
  --name easyssh-aks

# Verify cluster access
kubectl get nodes
```

#### Deploy EasySSH to AKS

```yaml
# easyssh-aks-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-pro-server
  namespace: easyssh
  labels:
    app: easyssh
    version: v0.3.0
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: easyssh
  template:
    metadata:
      labels:
        app: easyssh
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
    spec:
      serviceAccountName: easyssh-sa
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      containers:
        - name: easyssh-server
          image: easysshacr.azurecr.io/easyssh-pro-server:v0.3.0
          imagePullPolicy: Always
          ports:
            - name: http
              containerPort: 8080
              protocol: TCP
            - name: https
              containerPort: 8443
              protocol: TCP
            - name: metrics
              containerPort: 9090
              protocol: TCP
          env:
            - name: RUST_LOG
              value: "info"
            - name: EASYSSH_DATA_DIR
              value: "/data/easyssh"
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: easyssh-secrets
                  key: database-url
            - name: AZURE_KEY_VAULT_URI
              valueFrom:
                secretKeyRef:
                  name: easyssh-secrets
                  key: keyvault-uri
          resources:
            requests:
              memory: "256Mi"
              cpu: "250m"
            limits:
              memory: "512Mi"
              cpu: "500m"
          livenessProbe:
            httpGet:
              path: /health
              port: http
            initialDelaySeconds: 15
            periodSeconds: 20
          readinessProbe:
            httpGet:
              path: /ready
              port: http
            initialDelaySeconds: 5
            periodSeconds: 10
          securityContext:
            allowPrivilegeEscalation: false
            readOnlyRootFilesystem: true
            capabilities:
              drop:
                - ALL
          volumeMounts:
            - name: data
              mountPath: /data/easyssh
            - name: tmp
              mountPath: /tmp
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: easyssh-data-pvc
        - name: tmp
          emptyDir: {}
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchExpressions:
                    - key: app
                      operator: In
                      values:
                        - easyssh
                topologyKey: topology.kubernetes.io/zone
---
apiVersion: v1
kind: Service
metadata:
  name: easyssh-service
  namespace: easyssh
  annotations:
    service.beta.kubernetes.io/azure-load-balancer-internal: "false"
    service.beta.kubernetes.io/azure-dns-label-name: "easyssh-prod"
spec:
  type: LoadBalancer
  loadBalancerIP: <static-ip>  # Optional: Use static IP
  ports:
    - port: 80
      targetPort: http
      name: http
    - port: 443
      targetPort: https
      name: https
  selector:
    app: easyssh
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: easyssh-data-pvc
  namespace: easyssh
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: managed-csi
  resources:
    requests:
      storage: 50Gi
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: easyssh-hpa
  namespace: easyssh
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: easyssh-pro-server
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
---
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: easyssh-network-policy
  namespace: easyssh
spec:
  podSelector:
    matchLabels:
      app: easyssh
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress-nginx
      ports:
        - protocol: TCP
          port: 8080
        - protocol: TCP
          port: 8443
  egress:
    - to:
        - namespaceSelector:
            matchLabels:
              name: easyssh
      ports:
        - protocol: TCP
          port: 5432  # PostgreSQL
    - to:
        - namespaceSelector:
            matchLabels:
              name: easyssh
      ports:
        - protocol: TCP
          port: 6379  # Redis
```

#### Deploy to AKS

```bash
# Create namespace
kubectl create namespace easyssh
kubectl label namespace easyssh name=easyssh

# Create secrets from Azure Key Vault
kubectl create secret generic easyssh-secrets \
  --from-literal=database-url="<postgres-connection-string>" \
  --from-literal=keyvault-uri="<keyvault-uri>" \
  --namespace easyssh

# Apply deployment
kubectl apply -f easyssh-aks-deployment.yaml

# Verify deployment
kubectl get pods -n easyssh
kubectl get services -n easyssh
kubectl logs -f deployment/easyssh-pro-server -n easyssh
```

### 3.2 Azure Container Instances (ACI)

**Best for testing and prototyping.**

```bash
# Create Azure Container Registry (ACR)
az acr create \
  --resource-group easyssh-prod-rg \
  --name easysshacr \
  --sku Standard \
  --admin-enabled false

# Build and push image to ACR
az acr build \
  --registry easysshacr \
  --image easyssh-pro-server:v0.3.0 \
  --file docker/pro-server/Dockerfile .

# Create container instance
az container create \
  --resource-group easyssh-prod-rg \
  --name easyssh-server \
  --image easysshacr.azurecr.io/easyssh-pro-server:v0.3.0 \
  --registry-login-server easysshacr.azurecr.io \
  --registry-identity <managed-identity-id> \
  --cpu 2 \
  --memory 4 \
  --ports 8080 8443 \
  --environment-variables \
    RUST_LOG=info \
    EASYSSH_DATA_DIR=/data/easyssh \
  --secrets \
    database-url=<postgres-connection-string> \
  --secure-environment-variables \
    JWT_SECRET=<jwt-secret> \
  --dns-name-label easyssh-test \
  --assign-identity <managed-identity-id>

# Check container status
az container show \
  --resource-group easyssh-prod-rg \
  --name easyssh-server

# View container logs
az container logs \
  --resource-group easyssh-prod-rg \
  --name easyssh-server
```

### 3.3 Azure Container Apps

**Modern cloud-native deployment with autoscaling.**

```bash
# Create Container Apps environment
az containerapp env create \
  --name easyssh-env \
  --resource-group easyssh-prod-rg \
  --location eastus

# Create Container App
az containerapp create \
  --name easyssh-server \
  --resource-group easyssh-prod-rg \
  --environment easyssh-env \
  --image easysshacr.azurecr.io/easyssh-pro-server:v0.3.0 \
  --target-port 8080 \
  --ingress external \
  --min-replicas 1 \
  --max-replicas 10 \
  --scale-rule-name cpu-scale \
  --scale-rule-type cpu \
  --scale-rule-metadata type=Utilization value=70 \
  --env-vars \
    RUST_LOG=info \
    EASYSSH_DATA_DIR=/data/easyssh \
    DATABASE_URL=secretref:database-url \
  --secrets \
    database-url=<postgres-connection-string>

# Get app URL
az containerapp show \
  --name easyssh-server \
  --resource-group easyssh-prod-rg \
  --query properties.configuration.ingress.fqdn
```

### 3.4 Azure VM Deployment

**Traditional deployment for BYOD scenarios.**

```bash
# Create VM with custom image
az vm create \
  --resource-group easyssh-prod-rg \
  --name easyssh-vm \
  --image Ubuntu2204 \
  --size Standard_D2s_v3 \
  --admin-username easyssh \
  --ssh-key-value @~/.ssh/id_rsa.pub \
  --public-ip-address easyssh-pip \
  --public-ip-address-dns-name easyssh-server \
  --nsg easyssh-nsg \
  --nsg-rule ssh \
  --data-disk-sizes-gb 50 \
  --vnet-name easyssh-vnet \
  --subnet easyssh-subnet

# Configure VM
az vm run-command invoke \
  --resource-group easyssh-prod-rg \
  --name easyssh-vm \
  --command-id RunShellScript \
  --scripts @scripts/setup-easyssh.sh

# Open required ports
az network nsg rule create \
  --resource-group easyssh-prod-rg \
  --nsg-name easyssh-nsg \
  --name http-port \
  --protocol tcp \
  --priority 100 \
  --destination-port-ranges 8080

az network nsg rule create \
  --resource-group easyssh-prod-rg \
  --nsg-name easyssh-nsg \
  --name https-port \
  --protocol tcp \
  --priority 101 \
  --destination-port-ranges 8443
```

---

## 4. Infrastructure as Code

### 4.1 Bicep Template

```bicep
// easyssh-azure-deployment.bicep
// Target scope: subscription for resource group creation
targetScope = 'subscription'

@description('Location for all resources')
param location string = 'eastus'

@description('Environment name')
param environment string = 'prod'

@description('AKS cluster node count')
param nodeCount int = 3

@description('AKS node VM size')
param nodeVmSize string = 'Standard_D2s_v3'

@description('PostgreSQL server admin login')
param postgresAdminLogin string = 'easyssh_admin'

@description('PostgreSQL SKU')
param postgresSku string = 'GP_Standard_D2s_v3'

// Variables
var resourceGroupName = 'easyssh-${environment}-rg'
var aksClusterName = 'easyssh-aks-${environment}'
var acrName = 'easysshacr${environment}'
var keyVaultName = 'easysshkv${environment}'
var postgresServerName = 'easyssh-pg-${environment}'
var redisCacheName = 'easyssh-redis-${environment}'
var storageAccountName = 'easysshstg${environment}'
var logAnalyticsName = 'easyssh-logs-${environment}'

// Create resource group
resource resourceGroup 'Microsoft.Resources/resourceGroups@2023-07-01' = {
  name: resourceGroupName
  location: location
}

// Create Log Analytics Workspace
module logAnalytics 'modules/log-analytics.bicep' = {
  name: logAnalyticsName
  scope: resourceGroup
  params: {
    name: logAnalyticsName
    location: location
  }
}

// Create Azure Container Registry
module containerRegistry 'modules/container-registry.bicep' = {
  name: acrName
  scope: resourceGroup
  params: {
    name: acrName
    location: location
    sku: 'Standard'
  }
}

// Create Azure Key Vault
module keyVault 'modules/key-vault.bicep' = {
  name: keyVaultName
  scope: resourceGroup
  params: {
    name: keyVaultName
    location: location
    tenantId: subscription().tenantId
    objectId: aksCluster.properties.identityProfile.kubeletidentity.objectId
  }
  dependsOn: [
    aksCluster
  ]
}

// Create PostgreSQL Flexible Server
module postgres 'modules/postgresql.bicep' = {
  name: postgresServerName
  scope: resourceGroup
  params: {
    name: postgresServerName
    location: location
    skuName: postgresSku
    administratorLogin: postgresAdminLogin
    administratorLoginPassword: postgresAdminPassword
    version: '14'
    storageSizeGB: 128
  }
}

// Create Redis Cache
module redis 'modules/redis.bicep' = {
  name: redisCacheName
  scope: resourceGroup
  params: {
    name: redisCacheName
    location: location
    sku: 'Premium'
    capacity: 1
  }
}

// Create AKS Cluster
module aksCluster 'modules/aks.bicep' = {
  name: aksClusterName
  scope: resourceGroup
  params: {
    name: aksClusterName
    location: location
    nodeCount: nodeCount
    nodeVmSize: nodeVmSize
    dnsPrefix: 'easyssh-${environment}'
    networkPlugin: 'azure'
    enableMonitoring: true
    workspaceResourceId: logAnalytics.outputs.workspaceId
    acrId: containerRegistry.outputs.acrId
  }
  dependsOn: [
    logAnalytics
    containerRegistry
  ]
}

// Output important values
output aksClusterName string = aksClusterName
output aksClusterFqdn string = aksCluster.outputs.clusterFqdn
output acrLoginServer string = containerRegistry.outputs.loginServer
output keyVaultUri string = keyVault.outputs vaultUri
output postgresServerFqdn string = postgres.outputs.serverFqdn
output redisConnectionString string = redis.outputs.connectionString
```

#### Bicep Module: AKS Cluster

```bicep
// modules/aks.bicep
param name string
param location string
param nodeCount int
param nodeVmSize string
param dnsPrefix string
param networkPlugin string = 'azure'
param enableMonitoring bool = true
param workspaceResourceId string
param acrId string

resource aksCluster 'Microsoft.ContainerService/managedClusters@2023-11-01' = {
  name: name
  location: location
  sku: {
    name: 'Base'
    tier: 'Standard'
  }
  properties: {
    kubernetesVersion: '1.28'
    dnsPrefix: dnsPrefix
    agentPoolProfiles: [
      {
        name: 'agentpool'
        count: nodeCount
        vmSize: nodeVmSize
        osType: 'Linux'
        mode: 'System'
        enableAutoScaling: true
        minCount: nodeCount
        maxCount: 10
        zones: [
          '1'
          '2'
          '3'
        ]
        nodeTaints: []
        enableNodePublicIP: false
      }
    ]
    networkProfile: {
      networkPlugin: networkPlugin
      networkPolicy: 'calico'
      loadBalancerSku: 'standard'
      outboundType: 'loadBalancer'
    }
    addonProfiles: {
      omsAgent: {
        enabled: enableMonitoring
        config: {
          logAnalyticsWorkSpaceResourceId: workspaceResourceId
        }
      }
      azureKeyvaultSecretsProvider: {
        enabled: true
      }
    }
    identityProfile: {
      kubeletidentity: {
        resourceId: kubeletIdentityId
        clientId: kubeletIdentityClientId
        objectId: kubeletIdentityObjectId
      }
    }
    enableRBAC: true
    aadProfile: {
      enableAzureRBAC: true
      managed: true
      adminGroupObjectIDs: []
    }
  }
  identity: {
    type: 'SystemAssigned'
  }
}

// Grant ACR pull permission to AKS
resource acrPullRole 'Microsoft.Authorization/roleAssignments@2022-04-01' = {
  name: guid(acrId, aksCluster.properties.identityProfile.kubeletidentity.objectId, 'AcrPull')
  properties: {
    roleDefinitionId: '/subscriptions/${subscription().subscriptionId}/providers/Microsoft.Authorization/roleDefinitions/7f951dda-4ed3-4680-a7ca-43fe172d538d' // AcrPull role
    principalId: aksCluster.properties.identityProfile.kubeletidentity.objectId
    principalType: 'ServicePrincipal'
  }
}

output clusterFqdn string = aksCluster.properties.fqdn
output clusterId string = aksCluster.id
output kubeletIdentityObjectId string = aksCluster.properties.identityProfile.kubeletidentity.objectId
```

#### Bicep Module: Key Vault

```bicep
// modules/key-vault.bicep
param name string
param location string
param tenantId string
param objectId string

resource keyVault 'Microsoft.KeyVault/vaults@2023-07-01' = {
  name: name
  location: location
  properties: {
    tenantId: tenantId
    sku: {
      name: 'standard'
      family: 'A'
    }
    enableSoftDelete: true
    enablePurgeProtection: true
    softDeleteRetentionInDays: 90
    accessPolicies: [
      {
        tenantId: tenantId
        objectId: objectId
        permissions: {
          secrets: [
            'get'
            'list'
          ]
          keys: [
            'get'
            'list'
          ]
          certificates: [
            'get'
            'list'
          ]
        }
      }
    ]
    networkAcls: {
      bypass: 'AzureServices'
      defaultAction: 'Allow'
    }
  }
}

// Create secrets
resource databaseSecret 'Microsoft.KeyVault/vaults/secrets@2023-07-01' = {
  name: '${name}/database-url'
  properties: {
    value: databaseConnectionString
  }
}

resource jwtSecret 'Microsoft.KeyVault/vaults/secrets@2023-07-01' = {
  name: '${name}/jwt-secret'
  properties: {
    value: jwtSecretValue
  }
}

resource sslCertSecret 'Microsoft.KeyVault/vaults/secrets@2023-07-01' = {
  name: '${name}/ssl-certificate'
  properties: {
    value: sslCertificatePem
  }
}

output vaultUri string = keyVault.properties.vaultUri
output vaultId string = keyVault.id
```

### 4.2 Terraform Module

```hcl
# easyssh-azure-terraform/main.tf

# Configure Azure provider
terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 3.0"
    }
    azuread = {
      source  = "hashicorp/azuread"
      version = "~> 2.0"
    }
  }
  
  backend "azurerm" {
    resource_group_name  = "easyssh-terraform-state"
    storage_account_name = "easysshtfstate"
    container_name       = "tfstate"
    key                  = "easyssh-prod.tfstate"
  }
}

provider "azurerm" {
  features {
    key_vault {
      purge_soft_delete_on_destroy    = false
      recover_soft_deleted_key_vaults = true
    }
  }
}

provider "azuread" {}

# Variables
variable "location" {
  description = "Azure region for resources"
  type        = string
  default     = "eastus"
}

variable "environment" {
  description = "Deployment environment"
  type        = string
  default     = "prod"
}

variable "node_count" {
  description = "Number of AKS nodes"
  type        = number
  default     = 3
}

variable "node_vm_size" {
  description = "VM size for AKS nodes"
  type        = string
  default     = "Standard_D2s_v3"
}

# Local variables
locals {
  resource_group_name = "easyssh-${var.environment}-rg"
  aks_cluster_name    = "easyssh-aks-${var.environment}"
  acr_name            = "easysshacr${var.environment}"
  key_vault_name      = "easysshkv${var.environment}"
  postgres_name       = "easyssh-pg-${var.environment}"
  redis_name          = "easyssh-redis-${var.environment}"
  tags = {
    Environment = var.environment
    Project     = "EasySSH"
    ManagedBy   = "Terraform"
  }
}

# Resource Group
resource "azurerm_resource_group" "main" {
  name     = local.resource_group_name
  location = var.location
  tags     = local.tags
}

# Log Analytics Workspace
resource "azurerm_log_analytics_workspace" "main" {
  name                = "easyssh-logs-${var.environment}"
  location            = azurerm_resource_group.main.location
  resource_group_name = azurerm_resource_group.main.name
  sku                 = "PerGB2018"
  retention_in_days   = 30
  tags                = local.tags
}

# Azure Container Registry
resource "azurerm_container_registry" "main" {
  name                = local.acr_name
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location
  sku                 = "Standard"
  admin_enabled       = false
  tags                = local.tags
}

# Key Vault
resource "azurerm_key_vault" "main" {
  name                       = local.key_vault_name
  location                   = azurerm_resource_group.main.location
  resource_group_name        = azurerm_resource_group.main.name
  tenant_id                  = data.azurerm_client_config.current.tenant_id
  sku_name                   = "standard"
  soft_delete_retention_days = 90
  purge_protection_enabled   = true
  
  access_policy {
    tenant_id = data.azurerm_client_config.current.tenant_id
    object_id = data.azurerm_client_config.current.object_id
    
    secret_permissions = [
      "Get", "List", "Set", "Delete"
    ]
    
    key_permissions = [
      "Get", "List"
    ]
  }
  
  access_policy {
    tenant_id = data.azurerm_client_config.current.tenant_id
    object_id = azurerm_kubernetes_cluster.main.kubelet_identity[0].object_id
    
    secret_permissions = [
      "Get", "List"
    ]
  }
  
  tags = local.tags
}

# Key Vault Secrets
resource "azurerm_key_vault_secret" "database_url" {
  name         = "database-url"
  value        = azurerm_postgresql_flexible_server.main.fqdn
  key_vault_id = azurerm_key_vault.main.id
  
  depends_on = [
    azurerm_key_vault_access_policy.aks
  ]
}

resource "azurerm_key_vault_secret" "jwt_secret" {
  name         = "jwt-secret"
  value        = random_string.jwt_secret.result
  key_vault_id = azurerm_key_vault.main.id
  
  depends_on = [
    azurerm_key_vault_access_policy.aks
  ]
}

# PostgreSQL Flexible Server
resource "azurerm_postgresql_flexible_server" "main" {
  name                   = local.postgres_name
  resource_group_name    = azurerm_resource_group.main.name
  location               = azurerm_resource_group.main.location
  version                = "14"
  administrator_login    = "easyssh_admin"
  administrator_password = random_password.postgres.result
  sku_name               = "GP_Standard_D2s_v3"
  storage_mb             = 131072
  backup_retention_days  = 7
  geo_backup_enabled     = true
  
  authentication {
    active_directory_auth_enabled = true
    password_auth_enabled         = true
    tenant_id                     = data.azurerm_client_config.current.tenant_id
  }
  
  tags = local.tags
}

resource "azurerm_postgresql_flexible_server_database" "easyssh" {
  name      = "easyssh"
  server_id = azurerm_postgresql_flexible_server.main.id
}

# Redis Cache
resource "azurerm_redis_cache" "main" {
  name                = local.redis_name
  location            = azurerm_resource_group.main.location
  resource_group_name = azurerm_resource_group.main.name
  capacity            = 1
  family              = "P"
  sku_name            = "Premium"
  enable_non_ssl_port = false
  minimum_tls_version = "1.2"
  
  redis_configuration {
    maxmemory_policy = "volatile-lru"
  }
  
  tags = local.tags
}

# AKS Cluster
resource "azurerm_kubernetes_cluster" "main" {
  name                = local.aks_cluster_name
  location            = azurerm_resource_group.main.location
  resource_group_name = azurerm_resource_group.main.name
  dns_prefix          = "easyssh-${var.environment}"
  kubernetes_version  = "1.28"
  
  default_node_pool {
    name                = "agentpool"
    node_count          = var.node_count
    vm_size             = var.node_vm_size
    vnet_subnet_id      = azurerm_subnet.aks.id
    enable_auto_scaling = true
    min_count           = var.node_count
    max_count           = 10
    zones               = [1, 2, 3]
  }
  
  identity {
    type = "SystemAssigned"
  }
  
  network_profile {
    network_plugin    = "azure"
    network_policy    = "calico"
    load_balancer_sku = "standard"
    outbound_type     = "loadBalancer"
  }
  
  addon_profile {
    oms_agent {
      enabled                    = true
      log_analytics_workspace_id = azurerm_log_analytics_workspace.main.id
    }
    
    azure_keyvault_secrets_provider {
      enabled = true
    }
  }
  
  role_based_access_control_enabled = true
  azure_active_directory_role_based_access_control {
    managed            = true
    azure_rbac_enabled = true
  }
  
  tags = local.tags
}

# Grant ACR pull to AKS
resource "azurerm_role_assignment" "acr_pull" {
  principal_id                     = azurerm_kubernetes_cluster.main.kubelet_identity[0].object_id
  role_definition_name             = "AcrPull"
  scope                            = azurerm_container_registry.main.id
  skip_service_principal_aad_check = true
}

# Outputs
output "aks_cluster_name" {
  value = azurerm_kubernetes_cluster.main.name
}

output "aks_cluster_fqdn" {
  value = azurerm_kubernetes_cluster.main.fqdn
}

output "acr_login_server" {
  value = azurerm_container_registry.main.login_server
}

output "key_vault_uri" {
  value = azurerm_key_vault.main.vault_uri
}

output "postgres_fqdn" {
  value = azurerm_postgresql_flexible_server.main.fqdn
}

output "redis_connection_string" {
  value     = azurerm_redis_cache.main.primary_connection_string
  sensitive = true
}
```

---

## 5. Configuration

### 5.1 Azure AD Integration for Authentication

```yaml
# Azure AD App Registration
apiVersion: v1
kind: ConfigMap
metadata:
  name: easyssh-azure-ad-config
  namespace: easyssh
data:
  AZURE_AD_TENANT_ID: "<tenant-id>"
  AZURE_AD_CLIENT_ID: "<client-id>"
  AZURE_AD_AUTHORITY: "https://login.microsoftonline.com/<tenant-id>"
  AZURE_AD_SCOPES: "openid,profile,email"
---
# RBAC Configuration
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: easyssh-reader
rules:
- apiGroups: [""]
  resources: ["secrets", "configmaps"]
  verbs: ["get", "list"]
  resourceNames: ["easyssh-secrets", "easyssh-config"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: easyssh-reader-binding
subjects:
- kind: ServiceAccount
  name: easyssh-sa
  namespace: easyssh
roleRef:
  kind: ClusterRole
  name: easyssh-reader
  apiGroup: rbac.authorization.k8s.io
```

### 5.2 Azure Key Vault for Secrets

```bash
# Enable AKS Key Vault provider
az aks enable-addons \
  --addons azure-keyvault-secrets-provider \
  --name easyssh-aks \
  --resource-group easyssh-prod-rg

# Create Key Vault secrets
az keyvault secret set \
  --vault-name easysshkv \
  --name database-url \
  --value "<postgres-connection-string>"

az keyvault secret set \
  --vault-name easysshkv \
  --name jwt-secret \
  --value "$(openssl rand -base64 32)"

az keyvault secret set \
  --vault-name easysshkv \
  --name ssl-certificate \
  --value "$(cat cert.pem | base64)"

# Grant AKS access to Key Vault
az keyvault set-policy \
  --name easysshkv \
  --object-id <aks-kubelet-identity-object-id> \
  --secret-permissions get list
```

```yaml
# Kubernetes Secret Provider Class
apiVersion: secrets-store.csi.x-k8s.io/v1
kind: SecretProviderClass
metadata:
  name: easyssh-azure-kv
  namespace: easyssh
spec:
  provider: azure
  parameters:
    keyvaultName: "easysshkv"
    objects: |
      array:
        - |
          objectName: database-url
          objectType: secret
        - |
          objectName: jwt-secret
          objectType: secret
        - |
          objectName: ssl-certificate
          objectType: secret
    tenantId: "<tenant-id>"
---
# Pod mounting secrets
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-pro-server
  namespace: easyssh
spec:
  template:
    spec:
      containers:
        - name: easyssh-server
          volumeMounts:
            - name: secrets-store
              mountPath: "/mnt/secrets"
              readOnly: true
      volumes:
        - name: secrets-store
          csi:
            driver: secrets-store.csi.kubelets.io
            readOnly: true
            volumeAttributes:
              secretProviderClass: "easyssh-azure-kv"
```

### 5.3 Azure Container Registry (ACR)

```bash
# Create ACR
az acr create \
  --resource-group easyssh-prod-rg \
  --name easysshacr \
  --sku Premium \
  --location eastus

# Enable anonymous pull (optional, for public images)
az acr update \
  --name easysshacr \
  --anonymous-pull-enabled true

# Import existing image
az acr import \
  --name easysshacr \
  --source docker.io/library/easyssh-pro-server:latest \
  --image easyssh-pro-server:v0.3.0

# Build image in ACR
az acr build \
  --registry easysshacr \
  --image easyssh-pro-server:v0.3.0 \
  --file docker/pro-server/Dockerfile .

# Configure AKS to use ACR
az aks update \
  --name easyssh-aks \
  --resource-group easyssh-prod-rg \
  --attach-acr easysshacr
```

### 5.4 Load Balancer Setup

```yaml
# Azure Load Balancer Service Configuration
apiVersion: v1
kind: Service
metadata:
  name: easyssh-lb
  namespace: easyssh
  annotations:
    # Use Azure Standard Load Balancer
    service.beta.kubernetes.io/azure-load-balancer-internal: "false"
    # Static IP assignment
    service.beta.kubernetes.io/azure-load-balancer-resource-group: "easyssh-prod-rg"
    # Health probe configuration
    service.beta.kubernetes.io/azure-load-balancer-health-probe-request-path: "/health"
    # DNS label for public IP
    service.beta.kubernetes.io/azure-dns-label-name: "easyssh-prod"
spec:
  type: LoadBalancer
  loadBalancerIP: "<static-ip-address>"  # Optional: pre-created static IP
  externalTrafficPolicy: Local
  ports:
    - name: http
      port: 80
      targetPort: 8080
      protocol: TCP
    - name: https
      port: 443
      targetPort: 8443
      protocol: TCP
  selector:
    app: easyssh
```

```bash
# Create static public IP for Load Balancer
az network public-ip create \
  --resource-group easyssh-prod-rg \
  --name easyssh-pip \
  --sku Standard \
  --allocation-method Static \
  --dns-name easyssh-server \
  --location eastus

# Get the static IP
az network public-ip show \
  --resource-group easyssh-prod-rg \
  --name easyssh-pip \
  --query ipAddress
```

---

## 6. Security

### 6.1 Virtual Network Configuration

```hcl
# VNet and Subnets (Terraform)
resource "azurerm_virtual_network" "main" {
  name                = "easyssh-vnet"
  location            = azurerm_resource_group.main.location
  resource_group_name = azurerm_resource_group.main.name
  address_space       = ["10.0.0.0/16"]
  tags                = local.tags
}

resource "azurerm_subnet" "aks" {
  name                 = "aks-subnet"
  resource_group_name  = azurerm_resource_group.main.name
  virtual_network_name = azurerm_virtual_network.main.name
  address_prefixes     = ["10.0.0.0/20"]
  
  service_endpoints = [
    "Microsoft.Storage",
    "Microsoft.KeyVault",
    "Microsoft.Sql"
  ]
}

resource "azurerm_subnet" "postgres" {
  name                 = "postgres-subnet"
  resource_group_name  = azurerm_resource_group.main.name
  virtual_network_name = azurerm_virtual_network.main.name
  address_prefixes     = ["10.0.20.0/24"]
  
  delegation {
    name = "postgresql-delegation"
    service_delegation {
      name    = "Microsoft.DBforPostgreSQL/flexibleServers"
      actions = ["Microsoft.Network/virtualNetworks/subnets/join/action"]
    }
  }
}

resource "azurerm_subnet" "redis" {
  name                 = "redis-subnet"
  resource_group_name  = azurerm_resource_group.main.name
  virtual_network_name = azurerm_virtual_network.main.name
  address_prefixes     = ["10.0.21.0/24"]
}

resource "azurerm_subnet" "private_endpoints" {
  name                 = "private-endpoints-subnet"
  resource_group_name  = azurerm_resource_group.main.name
  virtual_network_name = azurerm_virtual_network.main.name
  address_prefixes     = ["10.0.22.0/24"]
  
  private_endpoint_network_policies_enabled = true
}
```

### 6.2 Network Security Groups

```bash
# Create NSG for AKS
az network nsg create \
  --resource-group easyssh-prod-rg \
  --name easyssh-aks-nsg

# Allow HTTPS traffic
az network nsg rule create \
  --resource-group easyssh-prod-rg \
  --nsg-name easyssh-aks-nsg \
  --name AllowHTTPS \
  --protocol tcp \
  --priority 100 \
  --destination-port-ranges 443 \
  --access Allow \
  --direction Inbound

# Allow health probes from Load Balancer
az network nsg rule create \
  --resource-group easyssh-prod-rg \
  --nsg-name easyssh-aks-nsg \
  --name AllowAzureLoadBalancer \
  --protocol "*" \
  --priority 200 \
  --source-address-prefixes "AzureLoadBalancer" \
  --access Allow \
  --direction Inbound

# Deny all other inbound traffic
az network nsg rule create \
  --resource-group easyssh-prod-rg \
  --nsg-name easyssh-aks-nsg \
  --name DenyAllInbound \
  --protocol "*" \
  --priority 300 \
  --access Deny \
  --direction Inbound
```

```yaml
# Kubernetes Network Policy
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: easyssh-network-policy
  namespace: easyssh
spec:
  podSelector:
    matchLabels:
      app: easyssh
  policyTypes:
    - Ingress
    - Egress
  ingress:
    # Allow ingress from Azure Load Balancer
    - from:
        - ipBlock:
            cidr: 0.0.0.0/0
            except:
              - 10.0.0.0/8
      ports:
        - protocol: TCP
          port: 8080
        - protocol: TCP
          port: 8443
    # Allow internal communication
    - from:
        - namespaceSelector:
            matchLabels:
              name: easyssh
      ports:
        - protocol: TCP
          port: 9090  # Metrics
  egress:
    # Allow PostgreSQL access
    - to:
        - ipBlock:
            cidr: 10.0.20.0/24
      ports:
        - protocol: TCP
          port: 5432
    # Allow Redis access
    - to:
        - ipBlock:
            cidr: 10.0.21.0/24
      ports:
        - protocol: TCP
          port: 6379
    # Allow Azure Key Vault
    - to:
        - ipBlock:
            cidr: 10.0.22.0/24
      ports:
        - protocol: TCP
          port: 443
    # Allow DNS resolution
    - to:
        - namespaceSelector: {}
      ports:
        - protocol: UDP
          port: 53
```

### 6.3 Managed Identity

```bash
# Create User Assigned Managed Identity
az identity create \
  --resource-group easyssh-prod-rg \
  --name easyssh-identity \
  --location eastus

# Get identity details
IDENTITY_ID=$(az identity show \
  --resource-group easyssh-prod-rg \
  --name easyssh-identity \
  --query id \
  --output tsv)

IDENTITY_PRINCIPAL_ID=$(az identity show \
  --resource-group easyssh-prod-rg \
  --name easyssh-identity \
  --query principalId \
  --output tsv)

# Assign roles to the identity
# Key Vault Secrets User
az role assignment create \
  --assignee $IDENTITY_PRINCIPAL_ID \
  --role "Key Vault Secrets User" \
  --scope "/subscriptions/<sub-id>/resourceGroups/easyssh-prod-rg/providers/Microsoft.KeyVault/vaults/easysshkv"

# Storage Blob Data Reader
az role assignment create \
  --assignee $IDENTITY_PRINCIPAL_ID \
  --role "Storage Blob Data Reader" \
  --scope "/subscriptions/<sub-id>/resourceGroups/easyssh-prod-rg/providers/Microsoft.Storage/storageAccounts/easysshstg"
```

```yaml
# Pod Managed Identity Configuration
apiVersion: aadpodidentity.k8s.io/v1
kind: AzureIdentity
metadata:
  name: easyssh-identity
  namespace: easyssh
spec:
  type: 0  # User-assigned managed identity
  resourceID: /subscriptions/<sub-id>/resourcegroups/easyssh-prod-rg/providers/Microsoft.ManagedIdentity/userAssignedIdentities/easyssh-identity
  clientID: <client-id>
---
apiVersion: aadpodidentity.k8s.io/v1
kind: AzureIdentityBinding
metadata:
  name: easyssh-identity-binding
  namespace: easyssh
spec:
  azureIdentity: easyssh-identity
  selector: easyssh
---
# Apply identity to pod
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-pro-server
  namespace: easyssh
spec:
  template:
    metadata:
      labels:
        aadpodidbinding: easyssh  # Links to AzureIdentityBinding
    spec:
      containers:
        - name: easyssh-server
          # Container can now use managed identity
```

---

## 7. Monitoring

### 7.1 Azure Monitor Integration

```bash
# Create Log Analytics Workspace
az monitor log-analytics workspace create \
  --resource-group easyssh-prod-rg \
  --workspace-name easyssh-logs \
  --location eastus

# Get workspace ID
WORKSPACE_ID=$(az monitor log-analytics workspace show \
  --resource-group easyssh-prod-rg \
  --workspace-name easyssh-logs \
  --query id \
  --output tsv)

# Enable AKS monitoring
az aks enable-addons \
  --addons monitoring \
  --name easyssh-aks \
  --resource-group easyssh-prod-rg \
  --workspace-resource-id $WORKSPACE_ID

# Create diagnostic settings for resources
az monitor diagnostic-settings create \
  --resource /subscriptions/<sub-id>/resourceGroups/easyssh-prod-rg/providers/Microsoft.ContainerService/managedClusters/easyssh-aks \
  --name easyssh-aks-logs \
  --workspace $WORKSPACE_ID \
  --logs '[{"category": "kube-apiserver", "enabled": true}, {"category": "kube-controller-manager", "enabled": true}, {"category": "kube-scheduler", "enabled": true}, {"category": "cluster-autoscaler", "enabled": true}]' \
  --metrics '[{"category": "AllMetrics", "enabled": true}]'
```

### 7.2 Log Analytics Workspace Queries

```kusto
// Container logs query
AzureDiagnostics
| where ResourceType == "CONTAINERS"
| where Category == "ContainerLog"
| where ContainerName_s == "easyssh-server"
| project TimeGenerated, Log_s, ContainerName_s
| order by TimeGenerated desc
| take 100

// AKS cluster health query
AzureDiagnostics
| where Category == "kube-apiserver"
| where OperationName == "get"
| project TimeGenerated, ResourceId, OperationName, Result_s
| summarize count() by Result_s, bin(TimeGenerated, 1h)
| render timechart

// Performance metrics query
InsightsMetrics
| where Namespace == "container"
| where Name == "cpuUsageNanoCores" or Name == "memoryRssBytes"
| where ContainerName == "easyssh-server"
| project TimeGenerated, Name, Val, ContainerName
| summarize avg(Val) by Name, bin(TimeGenerated, 5m)
| render timechart

// Health check failures
ContainerLog
| where ContainerName == "easyssh-server"
| where Log contains "health check failed"
| project TimeGenerated, Log
| order by TimeGenerated desc
```

### 7.3 Application Insights

```bash
# Create Application Insights
az monitor app-insights component create \
  --resource-group easyssh-prod-rg \
  --app easyssh-appinsights \
  --location eastus \
  --kind web \
  --application-type web

# Get instrumentation key
INSTRUMENTATION_KEY=$(az monitor app-insights component show \
  --resource-group easyssh-prod-rg \
  --app easyssh-appinsights \
  --query instrumentationKey \
  --output tsv)
```

```yaml
# Configure Application Insights in deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: easyssh-pro-server
  namespace: easyssh
spec:
  template:
    spec:
      containers:
        - name: easyssh-server
          env:
            - name: APPLICATIONINSIGHTS_CONNECTION_STRING
              value: "InstrumentationKey=<instrumentation-key>;IngestionEndpoint=https://eastus-0.in.applicationinsights.azure.com/"
            - name: APPLICATIONINSIGHTS_SAMPLING_PERCENTAGE
              value: "100"
```

### 7.4 Prometheus Integration

```yaml
# Prometheus scraping configuration
apiVersion: v1
kind: ConfigMap
metadata:
  name: prometheus-config
  namespace: monitoring
data:
  prometheus.yml: |
    global:
      scrape_interval: 15s
      evaluation_interval: 15s
    
    scrape_configs:
      - job_name: 'easyssh'
        kubernetes_sd_configs:
          - role: pod
            namespaces:
              names:
                - easyssh
        relabel_configs:
          - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
            action: keep
            regex: true
          - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
            action: replace
            target_label: __metrics_path__
            regex: (.+)
          - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
            action: replace
            regex: ([^:]+)(?::\d+)?;(\d+)
            replacement: $1:$2
            target_label: __address__
          - action: labelmap
            regex: __meta_kubernetes_pod_label_(.+)
          - source_labels: [__meta_kubernetes_namespace]
            action: replace
            target_label: namespace
          - source_labels: [__meta_kubernetes_pod_name]
            action: replace
            target_label: pod
---
# Azure Monitor Prometheus integration
apiVersion: monitoring.coreos.com/v1
kind: Prometheus
metadata:
  name: prometheus-azure
  namespace: monitoring
spec:
  serviceAccountName: prometheus-sa
  replicas: 1
  scrapeInterval: 30s
  evaluationInterval: 30s
  externalLabels:
    cluster: "easyssh-aks"
    environment: "prod"
  azureMonitorIntegration:
    enabled: true
    workspaceResourceId: "/subscriptions/<sub-id>/resourceGroups/easyssh-prod-rg/providers/Microsoft.OperationalInsights/workspaces/easyssh-logs"
```

---

## 8. Cost Estimation

### 8.1 Monthly Cost Breakdown (Production)

| Resource | SKU/Size | Monthly Cost (USD) | Notes |
|----------|----------|-------------------|-------|
| **AKS Cluster** | Standard tier | $73/mo | Base cluster fee |
| **AKS Node Pool** | 3 x Standard_D2s_v3 | $300/mo | ~$100/node |
| **Azure PostgreSQL** | GP_Standard_D2s_v3 | $150/mo | Flexible server |
| **Azure Redis** | Premium P1 | $200/mo | 6GB cache |
| **Azure Container Registry** | Premium | $50/mo | Geo-replication |
| **Azure Key Vault** | Standard | $3/mo | Secrets operations |
| **Azure Blob Storage** | 100GB + operations | $10/mo | Logs & backups |
| **Log Analytics** | PerGB2018, 30-day retention | $50/mo | ~2GB/day logs |
| **Application Insights** | Enterprise | $100/mo | Trace data |
| **Public IP** | Static Standard | $5/mo | Load balancer IP |
| **Network** | VNet, NSG, etc. | $10/mo | Network resources |
| **Total** | | **~$951/mo** | Production estimate |

### 8.2 Cost Optimization Recommendations

```bash
# Use spot instances for non-critical workloads
az aks nodepool add \
  --cluster-name easyssh-aks \
  --resource-group easyssh-prod-rg \
  --name spotpool \
  --node-count 2 \
  --node-vm-size Standard_D2s_v3 \
  --spot-max-price -1 \
  --priority Spot \
  --eviction-policy Delete

# Enable autoscaling to reduce idle costs
az aks update \
  --name easyssh-aks \
  --resource-group easyssh-prod-rg \
  --enable-cluster-autoscaler \
  --min-count 2 \
  --max-count 10

# Use reserved instances for stable workloads
# 1-year reserved: ~30% discount
# 3-year reserved: ~50% discount

# Set budgets and alerts
az consumption budget create \
  --budget-name easyssh-budget \
  --amount 1000 \
  --time-grain Monthly \
  --start-date 2026-04-01 \
  --end-date 2026-12-31 \
  --resource-group-filter easyssh-prod-rg
```

### 8.3 Development/Testing Cost

| Resource | SKU/Size | Monthly Cost (USD) | Notes |
|----------|----------|-------------------|-------|
| **AKS Cluster** | Free tier | $0 | Dev/test |
| **AKS Node Pool** | 1 x Standard_B2s | $30/mo | Burstable VM |
| **Azure PostgreSQL** | B_Standard_B1ms | $15/mo | Dev SKU |
| **Azure Redis** | Basic C0 | $16/mo | 250MB cache |
| **Azure Container Registry** | Basic | $5/mo | Limited storage |
| **Total** | | **~$66/mo** | Dev environment |

---

## Appendix

### A. Quick Deployment Commands

```bash
# Full production deployment script
#!/bin/bash
set -e

# Variables
RG="easyssh-prod-rg"
LOCATION="eastus"
AKS_NAME="easyssh-aks"
ACR_NAME="easysshacrprod"

# Create resource group
az group create --name $RG --location $LOCATION

# Create ACR
az acr create --resource-group $RG --name $ACR_NAME --sku Premium

# Create AKS with monitoring
az aks create \
  --resource-group $RG \
  --name $AKS_NAME \
  --node-count 3 \
  --node-vm-size Standard_D2s_v3 \
  --enable-managed-identity \
  --enable-addons monitoring \
  --generate-ssh-keys \
  --attach-acr $ACR_NAME

# Get credentials
az aks get-credentials --resource-group $RG --name $AKS_NAME

# Deploy application
kubectl apply -f easyssh-aks-deployment.yaml

# Verify
kubectl get pods -n easyssh
kubectl get services -n easyssh
```

### B. Troubleshooting Commands

```bash
# Check AKS cluster health
az aks show --resource-group easyssh-prod-rg --name easyssh-aks

# View pod logs
kubectl logs -n easyssh deployment/easyssh-pro-server --tail=100

# Check node status
kubectl get nodes -o wide

# Describe pod for issues
kubectl describe pod -n easyssh <pod-name>

# Check resource usage
kubectl top pods -n easyssh
kubectl top nodes

# Azure Key Vault access issues
az keyvault show --name easysshkv --query accessPolicies

# Network connectivity
kubectl exec -n easyssh <pod-name> -- nc -zv postgres-server 5432
```

### C. Useful Links

- [Azure AKS Documentation](https://docs.microsoft.com/en-us/azure/aks/)
- [Azure Container Registry](https://docs.microsoft.com/en-us/azure/container-registry/)
- [Azure Key Vault](https://docs.microsoft.com/en-us/azure/key-vault/)
- [Azure Monitor](https://docs.microsoft.com/en-us/azure/azure-monitor/)
- [Bicep Documentation](https://docs.microsoft.com/en-us/azure/azure-resource-manager/bicep/)
- [Terraform Azure Provider](https://registry.terraform.io/providers/hashicorp/azurerm/)

---

*EasySSH Azure Deployment Guide - Version 1.0 - 2026-04-03*