import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import type {
  K8sCluster,
  K8sPod,
  K8sNamespace,
  K8sNode,
  K8sDeployment,
  K8sService,
  K8sConfigMap,
  K8sSecret,
  K8sEvent,
  K8sPortForward,
  HelmRelease,
  HelmRepo,
  HelmChart,
  K8sResource,
  LogOptions,
  ExecOptions,
} from '../types';

// FFI functions (will be bound to Rust functions via Tauri)
declare global {
  interface Window {
    __TAURI__?: {
      invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
    };
  }
}

interface K8sState {
  // Clusters
  clusters: K8sCluster[];
  currentCluster: K8sCluster | null;
  isLoadingClusters: boolean;
  clusterError: string | null;

  // Namespaces
  namespaces: K8sNamespace[];
  currentNamespace: string;
  isLoadingNamespaces: boolean;

  // Pods
  pods: K8sPod[];
  selectedPod: K8sPod | null;
  isLoadingPods: boolean;
  podLogs: string;
  isLoadingLogs: boolean;
  logStream: ((log: string) => void) | null;

  // Nodes
  nodes: K8sNode[];
  isLoadingNodes: boolean;

  // Deployments
  deployments: K8sDeployment[];
  isLoadingDeployments: boolean;

  // Services
  services: K8sService[];
  isLoadingServices: boolean;

  // ConfigMaps
  configMaps: K8sConfigMap[];
  isLoadingConfigMaps: boolean;

  // Secrets
  secrets: K8sSecret[];
  isLoadingSecrets: boolean;

  // Events
  events: K8sEvent[];
  isLoadingEvents: boolean;
  isWatchingEvents: boolean;
  eventWatchUnsubscribe: (() => void) | null;

  // Port Forwards
  portForwards: K8sPortForward[];
  isLoadingPortForwards: boolean;

  // Helm
  helmReleases: HelmRelease[];
  helmRepos: HelmRepo[];
  isLoadingHelmReleases: boolean;
  isLoadingHelmRepos: boolean;

  // YAML Editor
  currentResource: K8sResource | null;
  isApplyingYaml: boolean;

  // Actions - Clusters
  loadClusters: () => Promise<void>;
  importKubeconfigFromPath: (path: string) => Promise<K8sCluster[]>;
  importKubeconfigFromContent: (content: string) => Promise<K8sCluster[]>;
  connectCluster: (clusterId: string) => Promise<void>;
  disconnectCluster: (clusterId: string) => Promise<void>;
  deleteCluster: (clusterId: string) => Promise<void>;
  selectCluster: (cluster: K8sCluster | null) => void;
  updateCluster: (cluster: K8sCluster) => Promise<void>;

  // Actions - Namespaces
  loadNamespaces: () => Promise<void>;
  setNamespace: (namespace: string) => Promise<void>;
  selectNamespace: (namespace: string) => void;

  // Actions - Pods
  loadPods: (labelSelector?: string) => Promise<void>;
  selectPod: (pod: K8sPod | null) => void;
  deletePod: (podName: string) => Promise<void>;
  restartPod: (podName: string) => Promise<void>;
  getPodLogs: (podName: string, options: LogOptions) => Promise<string>;
  streamPodLogs: (podName: string, options: LogOptions, onLog: (log: string) => void) => Promise<() => void>;
  execInPod: (podName: string, options: ExecOptions) => Promise<string>;

  // Actions - Nodes
  loadNodes: () => Promise<void>;

  // Actions - Deployments
  loadDeployments: () => Promise<void>;
  scaleDeployment: (deploymentName: string, replicas: number) => Promise<void>;

  // Actions - Services
  loadServices: () => Promise<void>;

  // Actions - ConfigMaps
  loadConfigMaps: () => Promise<void>;

  // Actions - Secrets
  loadSecrets: () => Promise<void>;

  // Actions - Events
  loadEvents: () => Promise<void>;
  watchEvents: () => Promise<void>;
  stopWatchingEvents: () => void;

  // Actions - Port Forwards
  loadPortForwards: () => Promise<void>;
  createPortForward: (podName: string, localPort: number, remotePort: number) => Promise<K8sPortForward>;
  createServicePortForward: (serviceName: string, localPort: number, remotePort: number) => Promise<K8sPortForward>;
  stopPortForward: (forwardId: string) => Promise<void>;
  deletePortForward: (forwardId: string) => Promise<void>;

  // Actions - Helm
  loadHelmReleases: () => Promise<void>;
  loadHelmRepos: () => Promise<void>;
  installHelmChart: (releaseName: string, chart: string, namespace: string, version?: string) => Promise<HelmRelease>;
  upgradeHelmRelease: (releaseName: string, chart: string, namespace: string, version?: string) => Promise<HelmRelease>;
  rollbackHelmRelease: (releaseName: string, namespace: string, revision: number) => Promise<HelmRelease>;
  uninstallHelmRelease: (releaseName: string, namespace: string) => Promise<void>;
  addHelmRepo: (name: string, url: string) => Promise<HelmRepo>;
  searchHelmCharts: (keyword: string) => Promise<HelmChart[]>;

  // Actions - YAML
  applyYaml: (yaml: string, namespace?: string) => Promise<K8sResource>;
  getResourceYaml: (kind: string, name: string, namespace: string) => Promise<string>;
  deleteResource: (kind: string, name: string, namespace: string) => Promise<void>;

  // Reset
  reset: () => void;
}

const initialState = {
  clusters: [],
  currentCluster: null,
  isLoadingClusters: false,
  clusterError: null,

  namespaces: [],
  currentNamespace: 'default',
  isLoadingNamespaces: false,

  pods: [],
  selectedPod: null,
  isLoadingPods: false,
  podLogs: '',
  isLoadingLogs: false,
  logStream: null,

  nodes: [],
  isLoadingNodes: false,

  deployments: [],
  isLoadingDeployments: false,

  services: [],
  isLoadingServices: false,

  configMaps: [],
  isLoadingConfigMaps: false,

  secrets: [],
  isLoadingSecrets: false,

  events: [],
  isLoadingEvents: false,
  isWatchingEvents: false,
  eventWatchUnsubscribe: null,

  portForwards: [],
  isLoadingPortForwards: false,

  helmReleases: [],
  helmRepos: [],
  isLoadingHelmReleases: false,
  isLoadingHelmRepos: false,

  currentResource: null,
  isApplyingYaml: false,
};

export const useK8sStore = create<K8sState>()(
  immer((set, get) => ({
    ...initialState,

    // Clusters
    loadClusters: async () => {
      set({ isLoadingClusters: true, clusterError: null });
      try {
        const clusters = await window.__TAURI__?.invoke<K8sCluster[]>('k8s_get_clusters') ?? [];
        set({ clusters, isLoadingClusters: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingClusters: false });
      }
    },

    importKubeconfigFromPath: async (path: string) => {
      set({ isLoadingClusters: true, clusterError: null });
      try {
        const clusters = await window.__TAURI__?.invoke<K8sCluster[]>('k8s_import_kubeconfig_path', { path }) ?? [];
        set(state => {
          state.clusters.push(...clusters);
          state.isLoadingClusters = false;
        });
        return clusters;
      } catch (error) {
        set({ clusterError: String(error), isLoadingClusters: false });
        throw error;
      }
    },

    importKubeconfigFromContent: async (content: string) => {
      set({ isLoadingClusters: true, clusterError: null });
      try {
        const clusters = await window.__TAURI__?.invoke<K8sCluster[]>('k8s_import_kubeconfig_content', { content }) ?? [];
        set(state => {
          state.clusters.push(...clusters);
          state.isLoadingClusters = false;
        });
        return clusters;
      } catch (error) {
        set({ clusterError: String(error), isLoadingClusters: false });
        throw error;
      }
    },

    connectCluster: async (clusterId: string) => {
      try {
        await window.__TAURI__?.invoke('k8s_connect_cluster', { clusterId });
        set(state => {
          const cluster = state.clusters.find(c => c.id === clusterId);
          if (cluster) {
            cluster.isConnected = true;
            cluster.lastConnectedAt = Date.now();
            state.currentCluster = cluster;
          }
        });
        // Load namespaces after connecting
        await get().loadNamespaces();
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    disconnectCluster: async (clusterId: string) => {
      try {
        await window.__TAURI__?.invoke('k8s_disconnect_cluster', { clusterId });
        set(state => {
          const cluster = state.clusters.find(c => c.id === clusterId);
          if (cluster) {
            cluster.isConnected = false;
          }
          if (state.currentCluster?.id === clusterId) {
            state.currentCluster = null;
          }
        });
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    deleteCluster: async (clusterId: string) => {
      try {
        await window.__TAURI__?.invoke('k8s_delete_cluster', { clusterId });
        set(state => {
          state.clusters = state.clusters.filter(c => c.id !== clusterId);
          if (state.currentCluster?.id === clusterId) {
            state.currentCluster = null;
          }
        });
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    selectCluster: (cluster: K8sCluster | null) => {
      set({ currentCluster: cluster });
      if (cluster) {
        get().loadNamespaces();
      }
    },

    updateCluster: async (cluster: K8sCluster) => {
      try {
        await window.__TAURI__?.invoke('k8s_update_cluster', { cluster });
        set(state => {
          const index = state.clusters.findIndex(c => c.id === cluster.id);
          if (index >= 0) {
            state.clusters[index] = cluster;
          }
          if (state.currentCluster?.id === cluster.id) {
            state.currentCluster = cluster;
          }
        });
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    // Namespaces
    loadNamespaces: async () => {
      const { currentCluster } = get();
      if (!currentCluster) return;

      set({ isLoadingNamespaces: true });
      try {
        const namespaces = await window.__TAURI__?.invoke<K8sNamespace[]>('k8s_get_namespaces', {
          clusterId: currentCluster.id,
        }) ?? [];
        set({ namespaces, isLoadingNamespaces: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingNamespaces: false });
      }
    },

    setNamespace: async (namespace: string) => {
      const { currentCluster } = get();
      if (!currentCluster) return;

      try {
        await window.__TAURI__?.invoke('k8s_set_namespace', {
          clusterId: currentCluster.id,
          namespace,
        });
        set(state => {
          state.currentNamespace = namespace;
          if (state.currentCluster) {
            state.currentCluster.currentNamespace = namespace;
          }
        });
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    selectNamespace: (namespace: string) => {
      set({ currentNamespace: namespace });
    },

    // Pods
    loadPods: async (labelSelector?: string) => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return;

      set({ isLoadingPods: true });
      try {
        const pods = await window.__TAURI__?.invoke<K8sPod[]>('k8s_get_pods', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
          labelSelector,
        }) ?? [];
        set({ pods, isLoadingPods: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingPods: false });
      }
    },

    selectPod: (pod: K8sPod | null) => {
      set({ selectedPod: pod });
    },

    deletePod: async (podName: string) => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return;

      try {
        await window.__TAURI__?.invoke('k8s_delete_pod', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
          podName,
        });
        set(state => {
          state.pods = state.pods.filter(p => p.name !== podName);
          if (state.selectedPod?.name === podName) {
            state.selectedPod = null;
          }
        });
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    restartPod: async (podName: string) => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return;

      try {
        await window.__TAURI__?.invoke('k8s_restart_pod', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
          podName,
        });
        // Reload pods after restart
        await get().loadPods();
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    getPodLogs: async (podName: string, options: LogOptions) => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return '';

      set({ isLoadingLogs: true });
      try {
        const logs = await window.__TAURI__?.invoke<string>('k8s_get_pod_logs', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
          podName,
          ...options,
        }) ?? '';
        set({ podLogs: logs, isLoadingLogs: false });
        return logs;
      } catch (error) {
        set({ clusterError: String(error), isLoadingLogs: false });
        throw error;
      }
    },

    streamPodLogs: async (podName: string, options: LogOptions, onLog: (log: string) => void) => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return () => {};

      // This would set up a WebSocket or Tauri event listener
      // For now, return a placeholder unsubscribe function
      set({ logStream: onLog });
      return () => {
        set({ logStream: null });
      };
    },

    execInPod: async (podName: string, options: ExecOptions) => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return '';

      try {
        const output = await window.__TAURI__?.invoke<string>('k8s_exec_in_pod', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
          podName,
          ...options,
        }) ?? '';
        return output;
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    // Nodes
    loadNodes: async () => {
      const { currentCluster } = get();
      if (!currentCluster) return;

      set({ isLoadingNodes: true });
      try {
        const nodes = await window.__TAURI__?.invoke<K8sNode[]>('k8s_get_nodes', {
          clusterId: currentCluster.id,
        }) ?? [];
        set({ nodes, isLoadingNodes: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingNodes: false });
      }
    },

    // Deployments
    loadDeployments: async () => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return;

      set({ isLoadingDeployments: true });
      try {
        const deployments = await window.__TAURI__?.invoke<K8sDeployment[]>('k8s_get_deployments', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
        }) ?? [];
        set({ deployments, isLoadingDeployments: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingDeployments: false });
      }
    },

    scaleDeployment: async (deploymentName: string, replicas: number) => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return;

      try {
        await window.__TAURI__?.invoke('k8s_scale_deployment', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
          deploymentName,
          replicas,
        });
        await get().loadDeployments();
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    // Services
    loadServices: async () => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return;

      set({ isLoadingServices: true });
      try {
        const services = await window.__TAURI__?.invoke<K8sService[]>('k8s_get_services', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
        }) ?? [];
        set({ services, isLoadingServices: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingServices: false });
      }
    },

    // ConfigMaps
    loadConfigMaps: async () => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return;

      set({ isLoadingConfigMaps: true });
      try {
        const configMaps = await window.__TAURI__?.invoke<K8sConfigMap[]>('k8s_get_configmaps', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
        }) ?? [];
        set({ configMaps, isLoadingConfigMaps: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingConfigMaps: false });
      }
    },

    // Secrets
    loadSecrets: async () => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return;

      set({ isLoadingSecrets: true });
      try {
        const secrets = await window.__TAURI__?.invoke<K8sSecret[]>('k8s_get_secrets', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
        }) ?? [];
        set({ secrets, isLoadingSecrets: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingSecrets: false });
      }
    },

    // Events
    loadEvents: async () => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return;

      set({ isLoadingEvents: true });
      try {
        const events = await window.__TAURI__?.invoke<K8sEvent[]>('k8s_get_events', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
        }) ?? [];
        set({ events, isLoadingEvents: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingEvents: false });
      }
    },

    watchEvents: async () => {
      // Set up event watching via Tauri events
      set({ isWatchingEvents: true });
    },

    stopWatchingEvents: () => {
      const { eventWatchUnsubscribe } = get();
      if (eventWatchUnsubscribe) {
        eventWatchUnsubscribe();
      }
      set({ isWatchingEvents: false, eventWatchUnsubscribe: null });
    },

    // Port Forwards
    loadPortForwards: async () => {
      set({ isLoadingPortForwards: true });
      try {
        const portForwards = await window.__TAURI__?.invoke<K8sPortForward[]>('k8s_get_port_forwards') ?? [];
        set({ portForwards, isLoadingPortForwards: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingPortForwards: false });
      }
    },

    createPortForward: async (podName: string, localPort: number, remotePort: number) => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) throw new Error('No cluster selected');

      try {
        const pf = await window.__TAURI__?.invoke<K8sPortForward>('k8s_create_port_forward', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
          podName,
          localPort,
          remotePort,
        });
        if (pf) {
          set(state => {
            state.portForwards.push(pf);
          });
        }
        return pf!;
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    createServicePortForward: async (serviceName: string, localPort: number, remotePort: number) => {
      // Similar to createPortForward but for services
      throw new Error('Not implemented');
    },

    stopPortForward: async (forwardId: string) => {
      try {
        await window.__TAURI__?.invoke('k8s_stop_port_forward', { forwardId });
        set(state => {
          const pf = state.portForwards.find(p => p.id === forwardId);
          if (pf) {
            pf.isActive = false;
          }
        });
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    deletePortForward: async (forwardId: string) => {
      try {
        await window.__TAURI__?.invoke('k8s_delete_port_forward', { forwardId });
        set(state => {
          state.portForwards = state.portForwards.filter(p => p.id !== forwardId);
        });
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    // Helm
    loadHelmReleases: async () => {
      const { currentCluster, currentNamespace } = get();
      if (!currentCluster) return;

      set({ isLoadingHelmReleases: true });
      try {
        const releases = await window.__TAURI__?.invoke<HelmRelease[]>('k8s_get_helm_releases', {
          clusterId: currentCluster.id,
          namespace: currentNamespace,
        }) ?? [];
        set({ helmReleases: releases, isLoadingHelmReleases: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingHelmReleases: false });
      }
    },

    loadHelmRepos: async () => {
      set({ isLoadingHelmRepos: true });
      try {
        const repos = await window.__TAURI__?.invoke<HelmRepo[]>('k8s_list_helm_repos') ?? [];
        set({ helmRepos: repos, isLoadingHelmRepos: false });
      } catch (error) {
        set({ clusterError: String(error), isLoadingHelmRepos: false });
      }
    },

    installHelmChart: async (releaseName: string, chart: string, namespace: string, version?: string) => {
      const { currentCluster } = get();
      if (!currentCluster) throw new Error('No cluster selected');

      try {
        const release = await window.__TAURI__?.invoke<HelmRelease>('k8s_helm_install', {
          clusterId: currentCluster.id,
          releaseName,
          chart,
          namespace,
          version,
        });
        if (release) {
          set(state => {
            state.helmReleases.push(release);
          });
        }
        return release!;
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    upgradeHelmRelease: async (releaseName: string, chart: string, namespace: string, version?: string) => {
      const { currentCluster } = get();
      if (!currentCluster) throw new Error('No cluster selected');

      try {
        const release = await window.__TAURI__?.invoke<HelmRelease>('k8s_helm_upgrade', {
          clusterId: currentCluster.id,
          releaseName,
          chart,
          namespace,
          version,
        });
        await get().loadHelmReleases();
        return release!;
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    rollbackHelmRelease: async (releaseName: string, namespace: string, revision: number) => {
      const { currentCluster } = get();
      if (!currentCluster) throw new Error('No cluster selected');

      try {
        const release = await window.__TAURI__?.invoke<HelmRelease>('k8s_helm_rollback', {
          clusterId: currentCluster.id,
          releaseName,
          namespace,
          revision,
        });
        await get().loadHelmReleases();
        return release!;
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    uninstallHelmRelease: async (releaseName: string, namespace: string) => {
      const { currentCluster } = get();
      if (!currentCluster) throw new Error('No cluster selected');

      try {
        await window.__TAURI__?.invoke('k8s_helm_uninstall', {
          clusterId: currentCluster.id,
          releaseName,
          namespace,
        });
        set(state => {
          state.helmReleases = state.helmReleases.filter(r => r.name !== releaseName);
        });
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    addHelmRepo: async (name: string, url: string) => {
      try {
        const repo = await window.__TAURI__?.invoke<HelmRepo>('k8s_add_helm_repo', { name, url });
        if (repo) {
          set(state => {
            state.helmRepos.push(repo);
          });
        }
        return repo!;
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    searchHelmCharts: async (keyword: string) => {
      try {
        const charts = await window.__TAURI__?.invoke<HelmChart[]>('k8s_search_helm_charts', { keyword }) ?? [];
        return charts;
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    // YAML
    applyYaml: async (yaml: string, namespace?: string) => {
      const { currentCluster } = get();
      if (!currentCluster) throw new Error('No cluster selected');

      set({ isApplyingYaml: true });
      try {
        const resource = await window.__TAURI__?.invoke<K8sResource>('k8s_apply_yaml', {
          clusterId: currentCluster.id,
          yaml,
          namespace: namespace || currentCluster.currentNamespace,
        });
        set({ isApplyingYaml: false });
        return resource!;
      } catch (error) {
        set({ clusterError: String(error), isApplyingYaml: false });
        throw error;
      }
    },

    getResourceYaml: async (kind: string, name: string, namespace: string) => {
      const { currentCluster } = get();
      if (!currentCluster) return '';

      try {
        const yaml = await window.__TAURI__?.invoke<string>('k8s_get_resource_yaml', {
          clusterId: currentCluster.id,
          kind,
          name,
          namespace,
        }) ?? '';
        return yaml;
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    deleteResource: async (kind: string, name: string, namespace: string) => {
      const { currentCluster } = get();
      if (!currentCluster) return;

      try {
        await window.__TAURI__?.invoke('k8s_delete_resource', {
          clusterId: currentCluster.id,
          kind,
          name,
          namespace,
        });
      } catch (error) {
        set({ clusterError: String(error) });
        throw error;
      }
    },

    reset: () => {
      const { eventWatchUnsubscribe, logStream } = get();
      if (eventWatchUnsubscribe) {
        eventWatchUnsubscribe();
      }
      if (logStream) {
        set({ logStream: null });
      }
      set(initialState);
    },
  }))
);
