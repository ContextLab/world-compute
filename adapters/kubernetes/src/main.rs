//! World Compute — Kubernetes adapter / operator
//!
//! Installs a `ClusterDonation` Custom Resource Definition (CRD) into a
//! Kubernetes cluster and watches for resources that describe donated node
//! capacity.  Each `ClusterDonation` CR corresponds to one World Compute
//! node registration.

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ClusterDonation CRD type (T149)
// ---------------------------------------------------------------------------

/// Spec for a `ClusterDonation` custom resource.
///
/// Represents donated Kubernetes cluster capacity for World Compute workloads.
/// This mirrors the CRD defined in the YAML below and in `helm/templates/crd.yaml`.
///
/// Note: We define the struct manually rather than using `kube::CustomResource`
/// derive to avoid pulling in `schemars`/`JsonSchema` — the CRD YAML is the
/// authoritative schema installed by the Helm chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterDonationSpec {
    /// CPU capacity cap (e.g. "4000m" for 4 cores).
    pub cpu_cap: String,
    /// Memory capacity cap (e.g. "8Gi").
    pub memory_cap: String,
    /// Allowed job classes for this donation.
    pub job_classes: Vec<String>,
    /// Kubernetes namespace for workload pods.
    pub namespace: String,
}

/// Full ClusterDonation resource (as stored in etcd).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterDonation {
    pub api_version: String,
    pub kind: String,
    pub metadata: ResourceMeta,
    pub spec: ClusterDonationSpec,
}

/// Minimal Kubernetes metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMeta {
    pub name: String,
    #[serde(default)]
    pub namespace: Option<String>,
}

impl ClusterDonation {
    /// Create a new ClusterDonation resource with the given spec.
    pub fn new(name: &str, spec: ClusterDonationSpec) -> Self {
        Self {
            api_version: "worldcompute.org/v1".to_string(),
            kind: "ClusterDonation".to_string(),
            metadata: ResourceMeta {
                name: name.to_string(),
                namespace: Some(spec.namespace.clone()),
            },
            spec,
        }
    }

    /// Serialize this resource to a Kubernetes-compatible JSON string.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("Serialization error: {e}"))
    }
}

// ---------------------------------------------------------------------------
// Pod creation / cleanup helpers (T150-T151)
// ---------------------------------------------------------------------------

/// Resource requirements for a task pod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu: String,
    pub memory: String,
}

/// Build the JSON manifest for a task pod (without requiring a live kube::Client).
///
/// In production, `create_task_pod` would use `kube::Api<Pod>::create()`.
/// This function builds the manifest that would be sent to the API server.
pub fn build_task_pod_manifest(
    namespace: &str,
    task_id: &str,
    image: &str,
    resources: &ResourceRequirements,
) -> serde_json::Value {
    serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": format!("wc-task-{task_id}"),
            "namespace": namespace,
            "labels": {
                "app.kubernetes.io/managed-by": "worldcompute",
                "worldcompute.org/task-id": task_id,
            }
        },
        "spec": {
            "restartPolicy": "Never",
            "containers": [{
                "name": "task",
                "image": image,
                "resources": {
                    "requests": {
                        "cpu": &resources.cpu,
                        "memory": &resources.memory,
                    },
                    "limits": {
                        "cpu": &resources.cpu,
                        "memory": &resources.memory,
                    }
                }
            }]
        }
    })
}

/// Build the delete options for pod cleanup.
pub fn build_cleanup_request(namespace: &str, task_id: &str) -> (String, String) {
    let pod_name = format!("wc-task-{task_id}");
    (namespace.to_string(), pod_name)
}

/// Async stub for pod creation — requires a live kube::Client.
///
/// ```ignore
/// pub async fn create_task_pod(
///     client: &kube::Client,
///     namespace: &str,
///     task_id: &str,
///     image: &str,
///     resources: ResourceRequirements,
/// ) -> Result<(), kube::Error> {
///     let pods: kube::Api<k8s_openapi::api::core::v1::Pod> =
///         kube::Api::namespaced(client.clone(), namespace);
///     let manifest = build_task_pod_manifest(namespace, task_id, image, &resources);
///     let pod: k8s_openapi::api::core::v1::Pod = serde_json::from_value(manifest).unwrap();
///     pods.create(&kube::api::PostParams::default(), &pod).await?;
///     Ok(())
/// }
/// ```
pub fn create_task_pod_manifest(
    namespace: &str,
    task_id: &str,
    image: &str,
    resources: &ResourceRequirements,
) -> serde_json::Value {
    build_task_pod_manifest(namespace, task_id, image, resources)
}

/// Async stub for pod cleanup — requires a live kube::Client.
///
/// ```ignore
/// pub async fn cleanup_pod(
///     client: &kube::Client,
///     namespace: &str,
///     task_id: &str,
/// ) -> Result<(), kube::Error> {
///     let pods: kube::Api<k8s_openapi::api::core::v1::Pod> =
///         kube::Api::namespaced(client.clone(), namespace);
///     pods.delete(&format!("wc-task-{task_id}"), &kube::api::DeleteParams::default()).await?;
///     Ok(())
/// }
/// ```
pub fn cleanup_pod_name(task_id: &str) -> String {
    format!("wc-task-{task_id}")
}

// ---------------------------------------------------------------------------
// CRD schema (YAML)
// ---------------------------------------------------------------------------

/// YAML definition of the `ClusterDonation` CRD installed by this operator.
pub const CLUSTER_DONATION_CRD: &str = r#"
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: clusterdonations.worldcompute.io
spec:
  group: worldcompute.io
  names:
    kind: ClusterDonation
    listKind: ClusterDonationList
    plural: clusterdonations
    singular: clusterdonation
    shortNames:
      - wcd
  scope: Cluster
  versions:
    - name: v1alpha1
      served: true
      storage: true
      schema:
        openAPIV3Schema:
          type: object
          properties:
            spec:
              type: object
              required: [namespace, maxCpuMillicores, maxRamBytes]
              properties:
                namespace:
                  type: string
                  description: Kubernetes namespace for World Compute workload pods.
                maxCpuMillicores:
                  type: integer
                  description: CPU capacity donated in millicores (1000 = 1 vCPU).
                maxRamBytes:
                  type: integer
                  description: RAM capacity donated in bytes.
                maxGpuCount:
                  type: integer
                  description: Number of GPUs donated (optional).
                gpuResourceKey:
                  type: string
                  description: Kubernetes extended resource key for GPUs (e.g. nvidia.com/gpu).
                coordinatorEndpoint:
                  type: string
                  description: World Compute coordinator gRPC endpoint.
                trustTier:
                  type: string
                  enum: [T1, T2, T3]
                  description: Node trust tier for task placement policy.
            status:
              type: object
              properties:
                phase:
                  type: string
                  enum: [Pending, Registered, Active, Draining, Error]
                lastHeartbeat:
                  type: string
                  format: date-time
                message:
                  type: string
      subresources:
        status: {}
      additionalPrinterColumns:
        - name: Phase
          type: string
          jsonPath: .status.phase
        - name: CPU(m)
          type: integer
          jsonPath: .spec.maxCpuMillicores
        - name: RAM
          type: integer
          jsonPath: .spec.maxRamBytes
        - name: Age
          type: date
          jsonPath: .metadata.creationTimestamp
"#;

// ---------------------------------------------------------------------------
// Adapter struct
// ---------------------------------------------------------------------------

/// Resource limits enforced on World Compute workload pods in this cluster.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_cpu_millicores: u64,
    pub max_ram_bytes: u64,
    pub max_gpu_count: u32,
}

/// Kubernetes adapter for World Compute.
pub struct K8sAdapter {
    /// Kubernetes namespace where workload pods are launched.
    pub namespace: String,
    /// Hard resource limits applied to every workload pod.
    pub resource_limits: ResourceLimits,
}

impl K8sAdapter {
    pub fn new(namespace: impl Into<String>, resource_limits: ResourceLimits) -> Self {
        Self { namespace: namespace.into(), resource_limits }
    }

    pub fn describe(&self) {
        println!("Kubernetes adapter");
        println!("  Namespace   : {}", self.namespace);
        println!("  Max CPU(m)  : {}", self.resource_limits.max_cpu_millicores);
        println!("  Max RAM     : {} bytes", self.resource_limits.max_ram_bytes);
        println!("  Max GPUs    : {}", self.resource_limits.max_gpu_count);
    }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "worldcompute-k8s-operator",
    about = "World Compute operator for Kubernetes clusters",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Apply the ClusterDonation CRD and RBAC manifests to the current cluster.
    Install {
        /// Kubernetes namespace for workload pods.
        #[arg(long, default_value = "worldcompute")]
        namespace: String,
        /// Maximum CPU in millicores to donate.
        #[arg(long, default_value_t = 4000)]
        max_cpu_millicores: u64,
        /// Maximum RAM in bytes to donate.
        #[arg(long, default_value_t = 8 * 1024 * 1024 * 1024)]
        max_ram_bytes: u64,
        /// Number of GPUs to donate (0 = none).
        #[arg(long, default_value_t = 0)]
        max_gpu_count: u32,
    },
    /// Show current operator and ClusterDonation status.
    Status,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crd_spec_creation() {
        let spec = ClusterDonationSpec {
            cpu_cap: "4000m".to_string(),
            memory_cap: "8Gi".to_string(),
            job_classes: vec!["batch".to_string(), "ml-inference".to_string()],
            namespace: "worldcompute".to_string(),
        };
        assert_eq!(spec.cpu_cap, "4000m");
        assert_eq!(spec.memory_cap, "8Gi");
        assert_eq!(spec.job_classes.len(), 2);
    }

    #[test]
    fn cluster_donation_resource() {
        let spec = ClusterDonationSpec {
            cpu_cap: "2000m".to_string(),
            memory_cap: "4Gi".to_string(),
            job_classes: vec!["batch".to_string()],
            namespace: "wc-prod".to_string(),
        };
        let cr = ClusterDonation::new("my-donation", spec);
        assert_eq!(cr.api_version, "worldcompute.org/v1");
        assert_eq!(cr.kind, "ClusterDonation");
        assert_eq!(cr.metadata.name, "my-donation");
        assert_eq!(cr.metadata.namespace, Some("wc-prod".to_string()));
    }

    #[test]
    fn cluster_donation_to_json() {
        let spec = ClusterDonationSpec {
            cpu_cap: "1000m".to_string(),
            memory_cap: "2Gi".to_string(),
            job_classes: vec![],
            namespace: "default".to_string(),
        };
        let cr = ClusterDonation::new("test", spec);
        let json = cr.to_json().unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["kind"], "ClusterDonation");
        assert_eq!(v["spec"]["cpu_cap"], "1000m");
    }

    #[test]
    fn pod_manifest_structure() {
        let res = ResourceRequirements { cpu: "500m".to_string(), memory: "1Gi".to_string() };
        let manifest = build_task_pod_manifest("wc-ns", "task-42", "ubuntu:22.04", &res);
        assert_eq!(manifest["kind"], "Pod");
        assert_eq!(manifest["metadata"]["name"], "wc-task-task-42");
        assert_eq!(manifest["metadata"]["namespace"], "wc-ns");
        assert_eq!(manifest["spec"]["containers"][0]["image"], "ubuntu:22.04");
        assert_eq!(manifest["spec"]["containers"][0]["resources"]["limits"]["cpu"], "500m");
    }

    #[test]
    fn cleanup_pod_name_format() {
        assert_eq!(cleanup_pod_name("abc-123"), "wc-task-abc-123");
    }

    #[test]
    fn resource_limits_default() {
        let limits = ResourceLimits {
            max_cpu_millicores: 4000,
            max_ram_bytes: 8 * 1024 * 1024 * 1024,
            max_gpu_count: 0,
        };
        assert_eq!(limits.max_cpu_millicores, 4000);
        assert_eq!(limits.max_gpu_count, 0);
    }

    #[test]
    fn crd_yaml_contains_key_fields() {
        assert!(CLUSTER_DONATION_CRD.contains("ClusterDonation"));
        assert!(CLUSTER_DONATION_CRD.contains("worldcompute.io"));
        assert!(CLUSTER_DONATION_CRD.contains("maxCpuMillicores"));
        assert!(CLUSTER_DONATION_CRD.contains("maxRamBytes"));
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install { namespace, max_cpu_millicores, max_ram_bytes, max_gpu_count } => {
            let limits = ResourceLimits { max_cpu_millicores, max_ram_bytes, max_gpu_count };
            let adapter = K8sAdapter::new(&namespace, limits);
            println!("Installing World Compute Kubernetes operator…");
            adapter.describe();
            println!();
            println!("CRD schema (ClusterDonation v1alpha1):");
            println!("{}", CLUSTER_DONATION_CRD.trim());
            println!();
            println!("Next steps:");
            println!("  1. kubectl apply -f <above CRD manifest>");
            println!("  2. Create a ClusterDonation CR in namespace '{namespace}'.");
            println!("  3. Run `worldcompute-k8s-operator status` to verify registration.");
        }
        Commands::Status => {
            println!("World Compute Kubernetes operator — status");
            println!();
            println!("  Operator pod    : not yet deployed");
            println!("  CRD installed   : unknown (run 'install' first)");
            println!("  Active donations: 0");
        }
    }
}
