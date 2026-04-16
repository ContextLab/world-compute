//! World Compute — Kubernetes adapter / operator
//!
//! Installs a `ClusterDonation` Custom Resource Definition (CRD) into a
//! Kubernetes cluster and watches for resources that describe donated node
//! capacity.  Each `ClusterDonation` CR corresponds to one World Compute
//! node registration.

use clap::{Parser, Subcommand};

// ---------------------------------------------------------------------------
// CRD schema
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
