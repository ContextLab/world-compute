//! World Compute — Cloud adapter
//!
//! Enables virtual machine instances on AWS, GCP, or Azure to join the
//! World Compute network as donor nodes.  The adapter runs inside the VM,
//! registers with a coordinator, and routes workload submissions to the
//! local container runtime.

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// AWS IMDSv2 identity parsing (T154)
// ---------------------------------------------------------------------------

/// Identity information extracted from the AWS instance identity document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsIdentity {
    pub instance_id: String,
    pub region: String,
    pub account_id: String,
}

/// Parse an AWS IMDSv2 instance identity document (JSON) into `AwsIdentity`.
///
/// The document is obtained from `http://169.254.169.254/latest/dynamic/instance-identity/document`
/// after acquiring a session token via PUT to the token endpoint.
pub fn parse_aws_identity_document(json: &str) -> Result<AwsIdentity, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {e}"))?;

    let instance_id = v
        .get("instanceId")
        .and_then(|v| v.as_str())
        .ok_or("Missing field: instanceId")?
        .to_string();

    let region =
        v.get("region").and_then(|v| v.as_str()).ok_or("Missing field: region")?.to_string();

    let account_id =
        v.get("accountId").and_then(|v| v.as_str()).ok_or("Missing field: accountId")?.to_string();

    Ok(AwsIdentity { instance_id, region, account_id })
}

// ---------------------------------------------------------------------------
// GCP metadata parsing (T155)
// ---------------------------------------------------------------------------

/// Identity information extracted from the GCP metadata server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpIdentity {
    pub instance_id: String,
    pub zone: String,
    pub project_id: String,
}

/// Parse a GCP metadata response (JSON) into `GcpIdentity`.
///
/// The instance identity token payload can be obtained from
/// `http://metadata.google.internal/computeMetadata/v1/instance/?recursive=true`
/// with the `Metadata-Flavor: Google` header.
pub fn parse_gcp_identity_token(json: &str) -> Result<GcpIdentity, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {e}"))?;

    let instance_id = v
        .get("id")
        .and_then(|v| v.as_u64().map(|n| n.to_string()).or_else(|| v.as_str().map(String::from)))
        .ok_or("Missing field: id")?;

    let zone = v.get("zone").and_then(|v| v.as_str()).ok_or("Missing field: zone")?.to_string();

    // zone is typically "projects/123456/zones/us-central1-a" — extract just the zone part
    let zone_short = zone.rsplit('/').next().unwrap_or(&zone).to_string();

    let project_id = v
        .get("project_id")
        .or_else(|| v.get("projectId"))
        .and_then(|v| v.as_str())
        .ok_or("Missing field: project_id")?
        .to_string();

    Ok(GcpIdentity { instance_id, zone: zone_short, project_id })
}

// ---------------------------------------------------------------------------
// Azure IMDS parsing (T156)
// ---------------------------------------------------------------------------

/// Identity information extracted from the Azure Instance Metadata Service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureIdentity {
    pub vm_id: String,
    pub location: String,
    pub subscription_id: String,
    pub resource_group: String,
}

/// Parse an Azure IMDS response (JSON) into `AzureIdentity`.
///
/// The document is obtained from
/// `http://169.254.169.254/metadata/instance?api-version=2021-02-01`
/// with the `Metadata: true` header.
pub fn parse_azure_identity(json: &str) -> Result<AzureIdentity, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {e}"))?;

    let compute = v.get("compute").unwrap_or(&v);

    let vm_id = compute
        .get("vmId")
        .and_then(|v| v.as_str())
        .ok_or("Missing field: compute.vmId")?
        .to_string();

    let location = compute
        .get("location")
        .and_then(|v| v.as_str())
        .ok_or("Missing field: compute.location")?
        .to_string();

    let subscription_id = compute
        .get("subscriptionId")
        .and_then(|v| v.as_str())
        .ok_or("Missing field: compute.subscriptionId")?
        .to_string();

    let resource_group = compute
        .get("resourceGroupName")
        .and_then(|v| v.as_str())
        .ok_or("Missing field: compute.resourceGroupName")?
        .to_string();

    Ok(AzureIdentity { vm_id, location, subscription_id, resource_group })
}

// ---------------------------------------------------------------------------
// Cloud provider enum
// ---------------------------------------------------------------------------

/// Supported public cloud providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    /// Amazon Web Services (EC2).
    Aws,
    /// Google Cloud Platform (Compute Engine).
    Gcp,
    /// Microsoft Azure (Virtual Machines).
    Azure,
}

impl CloudProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aws => "AWS",
            Self::Gcp => "GCP",
            Self::Azure => "Azure",
        }
    }
}

impl std::fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for CloudProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "aws" => Ok(Self::Aws),
            "gcp" => Ok(Self::Gcp),
            "azure" => Ok(Self::Azure),
            other => Err(format!("unknown provider '{other}'; expected aws, gcp, or azure")),
        }
    }
}

// ---------------------------------------------------------------------------
// Adapter struct
// ---------------------------------------------------------------------------

/// Cloud VM adapter for World Compute.
pub struct CloudAdapter {
    /// Cloud provider hosting this instance.
    pub provider: CloudProvider,
    /// Cloud-provider-assigned instance identifier.
    pub instance_id: String,
}

impl CloudAdapter {
    pub fn new(provider: CloudProvider, instance_id: impl Into<String>) -> Self {
        Self { provider, instance_id: instance_id.into() }
    }

    pub fn describe(&self) {
        println!("Cloud adapter");
        println!("  Provider    : {}", self.provider);
        println!("  Instance ID : {}", self.instance_id);
    }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "worldcompute-cloud-adapter",
    about = "World Compute adapter for cloud VM instances (AWS / GCP / Azure)",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Register this VM instance with the World Compute network.
    Join {
        /// Cloud provider: aws, gcp, or azure.
        #[arg(long)]
        provider: String,
        /// Cloud-provider instance identifier (e.g. i-0abc123def456789a).
        #[arg(long)]
        instance_id: String,
        /// World Compute coordinator gRPC endpoint.
        #[arg(long, default_value = "https://coordinator.worldcompute.io:443")]
        coordinator: String,
    },
    /// Show the current registration and health status of this instance.
    Status,
}

// `#[allow]` because `fn main` is declared after this test module by convention
// in this file; clippy's items-after-test-module lint would otherwise flag it.
#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    // --- AWS (T157) ---

    #[test]
    fn parse_aws_identity_valid() {
        let json = r#"{
            "instanceId": "i-0abc123def456789a",
            "region": "us-east-1",
            "accountId": "123456789012",
            "availabilityZone": "us-east-1a",
            "instanceType": "m5.xlarge"
        }"#;
        let id = parse_aws_identity_document(json).unwrap();
        assert_eq!(id.instance_id, "i-0abc123def456789a");
        assert_eq!(id.region, "us-east-1");
        assert_eq!(id.account_id, "123456789012");
    }

    #[test]
    fn parse_aws_identity_missing_field() {
        let json = r#"{"instanceId": "i-abc", "region": "us-west-2"}"#;
        assert!(parse_aws_identity_document(json).is_err());
    }

    #[test]
    fn parse_aws_identity_bad_json() {
        assert!(parse_aws_identity_document("not json").is_err());
    }

    // --- GCP (T157) ---

    #[test]
    fn parse_gcp_identity_valid() {
        let json = r#"{
            "id": 1234567890,
            "zone": "projects/my-project/zones/us-central1-a",
            "project_id": "my-project-id"
        }"#;
        let id = parse_gcp_identity_token(json).unwrap();
        assert_eq!(id.instance_id, "1234567890");
        assert_eq!(id.zone, "us-central1-a");
        assert_eq!(id.project_id, "my-project-id");
    }

    #[test]
    fn parse_gcp_identity_string_id() {
        let json = r#"{
            "id": "9876543210",
            "zone": "us-west1-b",
            "project_id": "proj-42"
        }"#;
        let id = parse_gcp_identity_token(json).unwrap();
        assert_eq!(id.instance_id, "9876543210");
        assert_eq!(id.zone, "us-west1-b");
    }

    #[test]
    fn parse_gcp_identity_missing_field() {
        let json = r#"{"id": 123}"#;
        assert!(parse_gcp_identity_token(json).is_err());
    }

    // --- Azure (T157) ---

    #[test]
    fn parse_azure_identity_valid() {
        let json = r#"{
            "compute": {
                "vmId": "vm-abc-123",
                "location": "eastus",
                "subscriptionId": "sub-1234",
                "resourceGroupName": "my-rg"
            }
        }"#;
        let id = parse_azure_identity(json).unwrap();
        assert_eq!(id.vm_id, "vm-abc-123");
        assert_eq!(id.location, "eastus");
        assert_eq!(id.subscription_id, "sub-1234");
        assert_eq!(id.resource_group, "my-rg");
    }

    #[test]
    fn parse_azure_identity_flat() {
        // Some IMDS responses may be flat (without compute wrapper)
        let json = r#"{
            "vmId": "vm-flat",
            "location": "westus2",
            "subscriptionId": "sub-flat",
            "resourceGroupName": "rg-flat"
        }"#;
        let id = parse_azure_identity(json).unwrap();
        assert_eq!(id.vm_id, "vm-flat");
        assert_eq!(id.location, "westus2");
    }

    #[test]
    fn parse_azure_identity_missing_field() {
        let json = r#"{"compute": {"vmId": "vm-1"}}"#;
        assert!(parse_azure_identity(json).is_err());
    }

    // --- CloudProvider ---

    #[test]
    fn cloud_provider_roundtrip() {
        assert_eq!("aws".parse::<CloudProvider>().unwrap(), CloudProvider::Aws);
        assert_eq!("GCP".parse::<CloudProvider>().unwrap(), CloudProvider::Gcp);
        assert_eq!("Azure".parse::<CloudProvider>().unwrap(), CloudProvider::Azure);
        assert!("other".parse::<CloudProvider>().is_err());
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Join { provider, instance_id, coordinator } => {
            let provider_enum = match provider.parse::<CloudProvider>() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            };
            let adapter = CloudAdapter::new(provider_enum, &instance_id);
            println!("Joining World Compute network…");
            adapter.describe();
            println!("  Coordinator : {coordinator}");
            println!();
            println!("Next steps:");
            println!("  1. Ensure outbound gRPC (port 443) to the coordinator is allowed.");
            println!("  2. The adapter will fetch a join token from the coordinator.");
            println!("  3. Run `worldcompute-cloud-adapter status` to verify registration.");
        }
        Commands::Status => {
            println!("World Compute cloud adapter — status");
            println!();
            println!("  Registration : not yet joined (run 'join' first)");
            println!("  Coordinator  : not connected");
            println!("  Tasks held   : 0");
        }
    }
}
