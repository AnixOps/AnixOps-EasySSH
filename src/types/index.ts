export type {
  AIProvider,
  AIModel,
  AIProviderConfig,
  ChatMessage,
  Conversation,
  QuickCommand,
  Attachment,
  AIAssistantSettings,
  VoiceSettings,
  SpeechRecognitionSettings,
  ExportFormat,
  ExportOptions,
  MessageRole,
  ContentPart,
  ContentPartType,
  VoiceProvider,
  StreamChunk,
} from './aiAssistant';

/**
 * Core type definitions for EasySSH
 * @module types
 */

// =============================================================================
// Server & Connection Types
// =============================================================================

/**
 * Authentication method for SSH connections
 */
export type AuthMethod = 'password' | 'key' | 'agent';

/**
 * SSH connection configuration
 */
export interface Server {
  /** Unique identifier */
  id: string;
  /** Display name */
  name: string;
  /** Hostname or IP address */
  host: string;
  /** SSH port (default: 22) */
  port: number;
  /** Username for authentication */
  username: string;
  /** Authentication method */
  authMethod: AuthMethod;
  /** Password (encrypted) - only for password auth */
  password?: string;
  /** Private key path - only for key auth */
  privateKey?: string;
  /** Key passphrase (encrypted) - only for key auth */
  keyPassphrase?: string;
  /** Group ID for organization */
  groupId?: string | null;
  /** Tags for filtering */
  tags: string[];
  /** Custom color indicator */
  color?: string;
  /** Notes/Description */
  notes?: string;
  /** Created timestamp */
  createdAt: number;
  /** Last modified timestamp */
  updatedAt: number;
  /** Last connected timestamp */
  lastConnectedAt?: number;
}

/**
 * Server group for organization
 */
export interface ServerGroup {
  /** Unique identifier */
  id: string;
  /** Display name */
  name: string;
  /** Parent group ID for nesting */
  parentId?: string | null;
  /** Custom color */
  color?: string;
  /** Icon identifier */
  icon?: string;
  /** Sort order */
  order: number;
  /** Created timestamp */
  createdAt: number;
}

// =============================================================================
// Session Types
// =============================================================================

/**
 * Connection session state
 */
export type SessionState = 'connecting' | 'connected' | 'disconnected' | 'error';

/**
 * Active connection session
 */
export interface Session {
  /** Unique session ID */
  id: string;
  /** Associated server ID */
  serverId: string;
  /** Session display name (may include index) */
  displayName: string;
  /** Current connection state */
  state: SessionState;
  /** Connection error message */
  error?: string;
  /** Session start time */
  startedAt: number;
  /** Last activity timestamp */
  lastActivityAt: number;
  /** Session index for multiple connections to same server */
  index: number;
}

// =============================================================================
// UI Component Types
// =============================================================================

/**
 * Theme preference
 */
export type Theme = 'light' | 'dark' | 'system';

/**
 * Size variants for components
 */
export type Size = 'xs' | 'sm' | 'md' | 'lg' | 'xl';

/**
 * Color variants for components
 */
export type ColorVariant =
  | 'default'
  | 'primary'
  | 'secondary'
  | 'success'
  | 'warning'
  | 'danger'
  | 'info';

/**
 * Sidebar item configuration
 */
export interface SidebarItem {
  /** Unique identifier */
  id: string;
  /** Display label */
  label: string;
  /** Icon component name */
  icon: string;
  /** Route or action */
  href?: string;
  /** Active state */
  isActive?: boolean;
  /** Badge count */
  badge?: number;
  /** Child items */
  children?: SidebarItem[];
  /** Disabled state */
  disabled?: boolean;
}

// =============================================================================
// Layout Types
// =============================================================================

/**
 * Panel layout configuration for split panes
 */
export interface PanelLayout {
  /** Panel ID */
  id: string;
  /** Panel type */
  type: 'terminal' | 'sftp' | 'monitor' | 'empty';
  /** Panel size (flex or px) */
  size: number | string;
  /** Minimum size */
  minSize?: number;
  /** Maximum size */
  maxSize?: number;
  /** Panel content configuration */
  config?: Record<string, unknown>;
}

/**
 * Layout direction
 */
export type LayoutDirection = 'horizontal' | 'vertical';

// =============================================================================
// Settings Types
// =============================================================================

/**
 * Application settings
 */
export interface AppSettings {
  /** UI theme */
  theme: Theme;
  /** Language code */
  language: string;
  /** Sidebar width in pixels */
  sidebarWidth: number;
  /** Whether sidebar is collapsed */
  sidebarCollapsed: boolean;
  /** Default terminal settings */
  terminal: TerminalSettings;
  /** Security settings */
  security: SecuritySettings;
}

/**
 * Terminal-specific settings
 */
export interface TerminalSettings {
  /** Font family */
  fontFamily: string;
  /** Font size in pixels */
  fontSize: number;
  /** Line height */
  lineHeight: number;
  /** Cursor style */
  cursorStyle: 'block' | 'line' | 'bar';
  /** Cursor blink */
  cursorBlink: boolean;
  /** Scrollback buffer size */
  scrollback: number;
  /** Enable WebGL renderer */
  useWebGL: boolean;
  /** Copy on select */
  copyOnSelect: boolean;
  /** Right click behavior */
  rightClickBehavior: 'contextMenu' | 'paste';
}

/**
 * Security settings
 */
export interface SecuritySettings {
  /** Lock app after inactivity (minutes, 0 = never) */
  autoLockTimeout: number;
  /** Clear clipboard on exit */
  clearClipboardOnExit: boolean;
  /** Confirm before bulk operations */
  confirmBulkOperations: boolean;
  /** Show connection notifications */
  showConnectionNotifications: boolean;
}

// =============================================================================
// Event Types
// =============================================================================

/**
 * Keyboard shortcut definition
 */
export interface KeyboardShortcut {
  /** Unique identifier */
  id: string;
  /** Display name */
  name: string;
  /** Key combination (e.g., 'Cmd+K') */
  key: string;
  /** Action description */
  description: string;
  /** Category for organization */
  category: string;
}

// =============================================================================
// API Response Types
// =============================================================================

/**
 * Standard API response wrapper
 */
export interface ApiResponse<T> {
  /** Success indicator */
  success: boolean;
  /** Response data */
  data?: T;
  /** Error message if failed */
  error?: string;
  /** Error code */
  code?: string;
}

// =============================================================================
// Kubernetes Types
// =============================================================================

/**
 * Kubernetes cluster configuration
 */
export interface K8sCluster {
  /** Unique identifier */
  id: string;
  /** Cluster display name */
  name: string;
  /** Path to kubeconfig file (optional if content is provided) */
  kubeconfigPath?: string;
  /** Kubeconfig content (optional if path is provided) */
  kubeconfigContent?: string;
  /** Current context name */
  context: string;
  /** Kubernetes API server URL */
  serverUrl: string;
  /** Currently selected namespace */
  currentNamespace: string;
  /** Connection status */
  isConnected: boolean;
  /** Last successful connection timestamp */
  lastConnectedAt?: number;
  /** Custom labels */
  labels: Record<string, string>;
  /** Tags for organization */
  tags: string[];
  /** Created timestamp */
  createdAt: number;
  /** Last modified timestamp */
  updatedAt: number;
}

/**
 * Kubernetes namespace
 */
export interface K8sNamespace {
  /** Namespace name */
  name: string;
  /** Namespace status (Active, Terminating) */
  status: string;
  /** Labels */
  labels: Record<string, string>;
  /** Annotations */
  annotations: Record<string, string>;
  /** Creation timestamp */
  createdAt: number;
}

/**
 * Pod status enumeration
 */
export type PodStatus =
  | 'Running'
  | 'Pending'
  | 'Succeeded'
  | 'Failed'
  | 'Unknown'
  | 'Terminating'
  | 'ContainerCreating'
  | 'ImagePullBackOff'
  | 'CrashLoopBackOff'
  | 'Error'
  | 'Completed'
  | 'Evicted'
  | 'OutOfMemory';

/**
 * Container state
 */
export type ContainerState =
  | { state: 'Running'; startedAt?: number }
  | { state: 'Waiting'; reason: string; message: string }
  | { state: 'Terminated'; exitCode: number; reason: string; finishedAt?: number }
  | { state: 'Unknown' };

/**
 * Kubernetes container information
 */
export interface K8sContainer {
  /** Container name */
  name: string;
  /** Container image */
  image: string;
  /** Ready status */
  ready: boolean;
  /** Restart count */
  restartCount: number;
  /** Current state */
  state: ContainerState;
  /** Exposed ports */
  ports: K8sContainerPort[];
  /** Resource requirements */
  resources?: K8sResourceRequirements;
}

/**
 * Container port
 */
export interface K8sContainerPort {
  /** Port name */
  name: string;
  /** Container port number */
  containerPort: number;
  /** Protocol (TCP/UDP) */
  protocol: string;
}

/**
 * Resource requirements
 */
export interface K8sResourceRequirements {
  /** Resource limits */
  limits: Record<string, string>;
  /** Resource requests */
  requests: Record<string, string>;
}

/**
 * Pod condition
 */
export interface K8sPodCondition {
  /** Condition type */
  conditionType: string;
  /** Status (True/False/Unknown) */
  status: string;
  /** Last probe time */
  lastProbeTime?: number;
  /** Last transition time */
  lastTransitionTime?: number;
  /** Reason */
  reason: string;
  /** Message */
  message: string;
}

/**
 * Pod volume
 */
export interface K8sVolume {
  /** Volume name */
  name: string;
  /** Volume type */
  volumeType: string;
  /** Volume source info */
  source: string;
}

/**
 * Resource usage metrics
 */
export interface K8sResourceUsage {
  /** CPU usage string (e.g., "100m") */
  cpuUsage: string;
  /** Memory usage string (e.g., "128Mi") */
  memoryUsage: string;
  /** CPU usage percentage */
  cpuPercent: number;
  /** Memory usage percentage */
  memoryPercent: number;
}

/**
 * Kubernetes Pod information
 */
export interface K8sPod {
  /** Pod name */
  name: string;
  /** Namespace */
  namespace: string;
  /** Pod status */
  status: PodStatus;
  /** Pod phase */
  phase: string;
  /** Total restart count */
  restarts: number;
  /** Node name */
  node: string;
  /** Pod IP address */
  podIp?: string;
  /** Labels */
  labels: Record<string, string>;
  /** Annotations */
  annotations: Record<string, string>;
  /** Containers */
  containers: K8sContainer[];
  /** Init containers */
  initContainers: K8sContainer[];
  /** Conditions */
  conditions: K8sPodCondition[];
  /** Volumes */
  volumes: K8sVolume[];
  /** Creation timestamp */
  createdAt: number;
  /** Start timestamp */
  startedAt?: number;
  /** Resource usage metrics */
  resourceUsage?: K8sResourceUsage;
}

/**
 * Node address
 */
export interface K8sNodeAddress {
  /** Address type (InternalIP, ExternalIP, Hostname) */
  addressType: string;
  /** Address value */
  address: string;
}

/**
 * Node condition
 */
export interface K8sNodeCondition {
  /** Condition type */
  conditionType: string;
  /** Status */
  status: string;
  /** Last heartbeat time */
  lastHeartbeatTime?: number;
  /** Last transition time */
  lastTransitionTime?: number;
  /** Reason */
  reason: string;
  /** Message */
  message: string;
}

/**
 * Node resource usage
 */
export interface K8sNodeResourceUsage {
  /** CPU core count */
  cpuCores: number;
  /** CPU usage percentage */
  cpuUsagePercent: number;
  /** Total memory */
  memoryTotal: string;
  /** Used memory */
  memoryUsed: string;
  /** Memory usage percentage */
  memoryUsagePercent: number;
  /** Current pod count */
  podCount: number;
  /** Maximum pod capacity */
  podCapacity: number;
}

/**
 * Kubernetes Node information
 */
export interface K8sNode {
  /** Node name */
  name: string;
  /** Node status */
  status: string;
  /** Node roles (master, worker, etc.) */
  roles: string[];
  /** Kubernetes version */
  version: string;
  /** OS image */
  osImage: string;
  /** Kernel version */
  kernelVersion: string;
  /** Container runtime version */
  containerRuntime: string;
  /** Labels */
  labels: Record<string, string>;
  /** Annotations */
  annotations: Record<string, string>;
  /** Node addresses */
  addresses: K8sNodeAddress[];
  /** Node capacity */
  capacity: Record<string, string>;
  /** Allocatable resources */
  allocatable: Record<string, string>;
  /** Node conditions */
  conditions: K8sNodeCondition[];
  /** Resource usage metrics */
  resourceUsage?: K8sNodeResourceUsage;
}

/**
 * Deployment condition
 */
export interface K8sDeploymentCondition {
  /** Condition type */
  conditionType: string;
  /** Status */
  status: string;
  /** Last update time */
  lastUpdateTime?: number;
  /** Last transition time */
  lastTransitionTime?: number;
  /** Reason */
  reason: string;
  /** Message */
  message: string;
}

/**
 * Kubernetes Deployment information
 */
export interface K8sDeployment {
  /** Deployment name */
  name: string;
  /** Namespace */
  namespace: string;
  /** Desired replicas */
  replicas: number;
  /** Available replicas */
  availableReplicas: number;
  /** Ready replicas */
  readyReplicas: number;
  /** Updated replicas */
  updatedReplicas: number;
  /** Labels */
  labels: Record<string, string>;
  /** Annotations */
  annotations: Record<string, string>;
  /** Pod selector */
  selector: Record<string, string>;
  /** Deployment strategy */
  strategy: string;
  /** Conditions */
  conditions: K8sDeploymentCondition[];
  /** Creation timestamp */
  createdAt: number;
  /** Last update timestamp */
  updatedAt: number;
}

/**
 * Service port
 */
export interface K8sServicePort {
  /** Port name */
  name: string;
  /** Service port */
  port: number;
  /** Target port (can be name or number) */
  targetPort: string;
  /** Node port (for NodePort services) */
  nodePort?: number;
  /** Protocol */
  protocol: string;
}

/**
 * Kubernetes Service information
 */
export interface K8sService {
  /** Service name */
  name: string;
  /** Namespace */
  namespace: string;
  /** Service type (ClusterIP, NodePort, LoadBalancer, ExternalName) */
  serviceType: string;
  /** Cluster IP */
  clusterIp: string;
  /** External IPs */
  externalIps: string[];
  /** Ports */
  ports: K8sServicePort[];
  /** Pod selector */
  selector: Record<string, string>;
  /** Labels */
  labels: Record<string, string>;
  /** Annotations */
  annotations: Record<string, string>;
  /** Creation timestamp */
  createdAt: number;
}

/**
 * Kubernetes ConfigMap information
 */
export interface K8sConfigMap {
  /** ConfigMap name */
  name: string;
  /** Namespace */
  namespace: string;
  /** Data entries */
  data: Record<string, string>;
  /** Binary data entries */
  binaryData: Record<string, string>;
  /** Labels */
  labels: Record<string, string>;
  /** Annotations */
  annotations: Record<string, string>;
  /** Creation timestamp */
  createdAt: number;
}

/**
 * Kubernetes Secret information
 */
export interface K8sSecret {
  /** Secret name */
  name: string;
  /** Namespace */
  namespace: string;
  /** Secret type (Opaque, kubernetes.io/tls, etc.) */
  secretType: string;
  /** Data keys (values not shown for security) */
  dataKeys: string[];
  /** Labels */
  labels: Record<string, string>;
  /** Annotations */
  annotations: Record<string, string>;
  /** Creation timestamp */
  createdAt: number;
}

/**
 * Object reference for events
 */
export interface K8sObjectReference {
  /** Object kind */
  kind: string;
  /** Object name */
  name: string;
  /** Namespace */
  namespace?: string;
  /** UID */
  uid?: string;
}

/**
 * Kubernetes Event information
 */
export interface K8sEvent {
  /** Event name */
  name: string;
  /** Namespace */
  namespace?: string;
  /** Event reason */
  reason: string;
  /** Event message */
  message: string;
  /** Event type (Normal, Warning) */
  eventType: string;
  /** Involved object */
  involvedObject: K8sObjectReference;
  /** Event count */
  count: number;
  /** First occurrence timestamp */
  firstTimestamp?: number;
  /** Last occurrence timestamp */
  lastTimestamp?: number;
  /** Event source */
  source: string;
}

/**
 * Port forward configuration
 */
export interface K8sPortForward {
  /** Forward ID */
  id: string;
  /** Cluster ID */
  clusterId: string;
  /** Namespace */
  namespace: string;
  /** Target pod name */
  podName: string;
  /** Target service name (if forwarding to service) */
  serviceName?: string;
  /** Local port */
  localPort: number;
  /** Remote port */
  remotePort: number;
  /** Protocol (TCP/UDP) */
  protocol: string;
  /** Active status */
  isActive: boolean;
  /** Creation timestamp */
  createdAt: number;
}

/**
 * Helm maintainer information
 */
export interface HelmMaintainer {
  /** Maintainer name */
  name: string;
  /** Maintainer email */
  email: string;
}

/**
 * Helm chart information
 */
export interface HelmChart {
  /** Chart name */
  name: string;
  /** Chart version */
  version: string;
  /** App version */
  appVersion: string;
  /** Description */
  description: string;
  /** Keywords */
  keywords: string[];
  /** Maintainers */
  maintainers: HelmMaintainer[];
  /** Icon URL */
  icon: string;
  /** Chart URLs */
  urls: string[];
}

/**
 * Helm release information
 */
export interface HelmRelease {
  /** Release name */
  name: string;
  /** Namespace */
  namespace: string;
  /** Chart name */
  chart: string;
  /** Chart version */
  chartVersion: string;
  /** App version */
  appVersion: string;
  /** Revision number */
  revision: number;
  /** Release status */
  status: string;
  /** Last update timestamp */
  updatedAt: number;
  /** Values used for installation */
  values: Record<string, unknown>;
}

/**
 * Helm repository
 */
export interface HelmRepo {
  /** Repository name */
  name: string;
  /** Repository URL */
  url: string;
}

/**
 * Resource metadata
 */
export interface K8sResourceMetadata {
  /** Resource name */
  name: string;
  /** Namespace */
  namespace?: string;
  /** Labels */
  labels?: Record<string, string>;
  /** Annotations */
  annotations?: Record<string, string>;
  /** UID */
  uid?: string;
  /** Resource version */
  resourceVersion?: string;
  /** Creation timestamp */
  creationTimestamp?: number;
}

/**
 * Kubernetes Resource (for YAML editor)
 */
export interface K8sResource {
  /** API version */
  apiVersion: string;
  /** Kind */
  kind: string;
  /** Metadata */
  metadata: K8sResourceMetadata;
  /** Spec */
  spec?: Record<string, unknown>;
  /** Status */
  status?: Record<string, unknown>;
}

/**
 * Log options for kubectl logs
 */
export interface LogOptions {
  /** Follow logs (kubectl logs -f) */
  follow: boolean;
  /** Number of lines from the end */
  tailLines?: number;
  /** Show logs since relative time in seconds */
  sinceSeconds?: number;
  /** Include timestamps */
  timestamps: boolean;
  /** Show previous container logs */
  previous: boolean;
  /** Container name (for multi-container pods) */
  container?: string;
}

/**
 * Exec options for kubectl exec
 */
export interface ExecOptions {
  /** Container name (for multi-container pods) */
  container?: string;
  /** Enable stdin */
  stdin: boolean;
  /** Allocate TTY */
  tty: boolean;
  /** Command to execute */
  command: string[];
}

/**
 * Pagination parameters
 */
export interface PaginationParams {
  /** Page number (1-based) */
  page: number;
  /** Items per page */
  limit: number;
  /** Sort field */
  sortBy?: string;
  /** Sort direction */
  sortOrder?: 'asc' | 'desc';
}

/**
 * Paginated response
 */
export interface PaginatedResponse<T> {
  /** Items for current page */
  items: T[];
  /** Total item count */
  total: number;
  /** Current page */
  page: number;
  /** Items per page */
  limit: number;
  /** Total pages */
  totalPages: number;
}
