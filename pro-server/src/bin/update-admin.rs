//! Admin CLI for EasySSH Update Server
//!
//! Usage:
//!   cargo run --bin update-admin -- release create 0.4.0 100 stable
//!   cargo run --bin update-admin -- rollout set 0.4.0 50
//!   cargo run --bin update-admin -- rollout status 0.4.0

use clap::{Parser, Subcommand};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "update-admin")]
#[command(about = "EasySSH Update Server Admin CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// API base URL
    #[arg(short, long, default_value = "https://updates.easyssh.dev/api/v1")]
    api_url: String,

    /// Admin API token
    #[arg(short, long, env = "EASYSSH_ADMIN_TOKEN")]
    token: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage releases
    #[command(subcommand)]
    Release(ReleaseCommands),

    /// Manage rollouts
    #[command(subcommand)]
    Rollout(RolloutCommands),

    /// Manage feature flags
    #[command(subcommand)]
    Feature(FeatureCommands),

    /// View statistics
    Stats {
        /// Version to query (optional)
        version: Option<String>,
    },

    /// A/B testing management
    #[command(subcommand)]
    AbTest(AbTestCommands),
}

#[derive(Subcommand)]
enum ReleaseCommands {
    /// Create a new release
    Create {
        /// Version string (e.g., "0.4.0")
        version: String,
        /// Build number
        build_number: u32,
        /// Release channel
        #[arg(default_value = "stable")]
        channel: String,
        /// Force update (critical security fix)
        #[arg(long)]
        force_update: bool,
        /// Minimum required version
        #[arg(long)]
        min_version: Option<String>,
        /// Release notes file
        #[arg(long)]
        notes_file: Option<String>,
    },

    /// List releases
    List {
        /// Filter by channel
        #[arg(short, long)]
        channel: Option<String>,
    },

    /// Get release details
    Get {
        version: String,
    },

    /// Update release
    Update {
        version: String,
        /// New rollout percentage
        #[arg(short, long)]
        rollout: Option<u8>,
        /// Set force update flag
        #[arg(long)]
        force_update: Option<bool>,
    },

    /// Delete release
    Delete {
        version: String,
        /// Force deletion
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum RolloutCommands {
    /// Set rollout percentage
    Set {
        version: String,
        percentage: u8,
    },

    /// Get rollout status
    Status {
        version: String,
    },

    /// Pause rollout
    Pause {
        version: String,
    },

    /// Resume rollout
    Resume {
        version: String,
    },

    /// Complete rollout (100%)
    Complete {
        version: String,
    },
}

#[derive(Subcommand)]
enum FeatureCommands {
    /// Create feature flag
    Create {
        name: String,
        #[arg(short, long)]
        rollout: u8,
        #[arg(short, long)]
        target_groups: Vec<String>,
    },

    /// Enable feature flag
    Enable {
        name: String,
    },

    /// Disable feature flag
    Disable {
        name: String,
    },

    /// List feature flags
    List,

    /// Update rollout percentage
    Rollout {
        name: String,
        percentage: u8,
    },
}

#[derive(Subcommand)]
enum AbTestCommands {
    /// Create A/B test group
    CreateGroup {
        name: String,
        #[arg(short, long)]
        features: Vec<String>,
    },

    /// Assign user to group
    Assign {
        install_id: String,
        group: String,
    },

    /// Get group stats
    Stats {
        group: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct ReleaseRequest {
    version: String,
    build_number: u32,
    channel: String,
    force_update: bool,
    min_version: Option<String>,
    release_notes: String,
    rollout_percentage: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReleaseResponse {
    version: String,
    build_number: u32,
    channel: String,
    rollout_percentage: u8,
    downloads: u64,
    successful_installs: u64,
    failed_installs: u64,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StatsResponse {
    version: String,
    rollout_percentage: u8,
    downloads: u64,
    successful_installs: u64,
    failed_installs: u64,
    error_rate: f32,
    rollback_rate: f32,
    errors: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RolloutRequest {
    percentage: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeatureFlagRequest {
    name: String,
    enabled: bool,
    rollout_percentage: u8,
    target_groups: Vec<String>,
}

struct UpdateAdmin {
    client: Client,
    base_url: String,
    token: String,
}

impl UpdateAdmin {
    fn new(base_url: String, token: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            token,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    async fn create_release(&self, req: ReleaseRequest) -> anyhow::Result<ReleaseResponse> {
        let response = self.client
            .post(format!("{}/admin/releases", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&req)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(anyhow::anyhow!("Failed to create release: {}", response.status()))
        }
    }

    async fn list_releases(&self, channel: Option<String>) -> anyhow::Result<Vec<ReleaseResponse>> {
        let mut url = format!("{}/admin/releases", self.base_url);

        if let Some(ch) = channel {
            url.push_str(&format!("?channel={}", ch));
        }

        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(anyhow::anyhow!("Failed to list releases"))
        }
    }

    async fn get_release(&self, version: &str) -> anyhow::Result<ReleaseResponse> {
        let response = self.client
            .get(format!("{}/admin/releases/{}", self.base_url, version))
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(anyhow::anyhow!("Release not found"))
        }
    }

    async fn update_release(
        &self,
        version: &str,
        rollout: Option<u8>,
        force_update: Option<bool>,
    ) -> anyhow::Result<()> {
        let mut body = std::collections::HashMap::new();

        if let Some(pct) = rollout {
            body.insert("rollout_percentage", pct.to_string());
        }

        if let Some(force) = force_update {
            body.insert("force_update", force.to_string());
        }

        let response = self.client
            .patch(format!("{}/admin/releases/{}", self.base_url, version))
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to update release"))
        }
    }

    async fn delete_release(&self, version: &str, force: bool) -> anyhow::Result<()> {
        let url = format!("{}/admin/releases/{}?force={}", self.base_url, version, force);

        let response = self.client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to delete release"))
        }
    }

    async fn set_rollout(&self, version: &str, percentage: u8) -> anyhow::Result<()> {
        let req = RolloutRequest { percentage };

        let response = self.client
            .post(format!("{}/admin/releases/{}/rollout", self.base_url, version))
            .header("Authorization", self.auth_header())
            .json(&req)
            .send()
            .await?;

        if response.status().is_success() {
            println!("✅ Rollout for {} set to {}%", version, percentage);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to set rollout"))
        }
    }

    async fn get_stats(&self, version: Option<&str>) -> anyhow::Result<StatsResponse> {
        let url = if let Some(v) = version {
            format!("{}/admin/stats/{}", self.base_url, v)
        } else {
            format!("{}/admin/stats", self.base_url)
        };

        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(anyhow::anyhow!("Failed to get stats"))
        }
    }

    async fn create_feature_flag(&self, req: FeatureFlagRequest) -> anyhow::Result<()> {
        let response = self.client
            .post(format!("{}/admin/features", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&req)
            .send()
            .await?;

        if response.status().is_success() {
            println!("✅ Feature flag '{}' created", req.name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to create feature flag"))
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let admin = UpdateAdmin::new(cli.api_url, cli.token);

    match cli.command {
        Commands::Release(cmd) => match cmd {
            ReleaseCommands::Create {
                version,
                build_number,
                channel,
                force_update,
                min_version,
                notes_file,
            } => {
                let release_notes = if let Some(file) = notes_file {
                    tokio::fs::read_to_string(file).await?
                } else {
                    format!("Release {} build {}", version, build_number)
                };

                let req = ReleaseRequest {
                    version,
                    build_number,
                    channel,
                    force_update,
                    min_version,
                    release_notes,
                    rollout_percentage: 5, // Start at 5%
                };

                let release = admin.create_release(req).await?;
                println!("✅ Release {} created successfully!", release.version);
                println!("   Build: {}", release.build_number);
                println!("   Channel: {}", release.channel);
                println!("   Rollout: {}%", release.rollout_percentage);
            }

            ReleaseCommands::List { channel } => {
                let releases = admin.list_releases(channel).await?;

                println!("{:<12} {:<10} {:<10} {:<12} {:<20}",
                    "Version", "Build", "Channel", "Rollout", "Downloads");
                println!("{}", "-".repeat(70));

                for release in releases {
                    println!("{:<12} {:<10} {:<10} {:<12} {:<20}",
                        release.version,
                        release.build_number,
                        release.channel,
                        format!("{}%", release.rollout_percentage),
                        release.downloads,
                    );
                }
            }

            ReleaseCommands::Get { version } => {
                let release = admin.get_release(&version).await?;

                println!("Release Details");
                println!("===============");
                println!("Version: {}", release.version);
                println!("Build: {}", release.build_number);
                println!("Channel: {}", release.channel);
                println!("Rollout: {}%", release.rollout_percentage);
                println!("Downloads: {}", release.downloads);
                println!("Successful: {}", release.successful_installs);
                println!("Failed: {}", release.failed_installs);
                println!("Created: {}", release.created_at);
            }

            ReleaseCommands::Update {
                version,
                rollout,
                force_update,
            } => {
                admin.update_release(&version, rollout, force_update).await?;
                println!("✅ Release {} updated", version);
            }

            ReleaseCommands::Delete { version, force } => {
                if !force {
                    println!("⚠️  This will delete release {}. Use --force to confirm.", version);
                    return Ok(());
                }

                admin.delete_release(&version, force).await?;
                println!("✅ Release {} deleted", version);
            }
        },

        Commands::Rollout(cmd) => match cmd {
            RolloutCommands::Set { version, percentage } => {
                admin.set_rollout(&version, percentage).await?;
            }

            RolloutCommands::Status { version } => {
                let stats = admin.get_stats(Some(&version)).await?;

                println!("Rollout Status for {}", version);
                println!("========================");
                println!("Rollout: {}%", stats.rollout_percentage);
                println!("Downloads: {}", stats.downloads);
                println!("Success: {}", stats.successful_installs);
                println!("Failed: {}", stats.failed_installs);
                println!("Error Rate: {:.2}%", stats.error_rate);
                println!("Rollback Rate: {:.2}%", stats.rollback_rate);

                if stats.error_rate > 5.0 {
                    println!("\n⚠️  WARNING: Error rate exceeds 5%!");
                } else if stats.error_rate > 1.0 {
                    println!("\n⚠️  Caution: Error rate is elevated.");
                } else {
                    println!("\n✅ Rollout is healthy.");
                }
            }

            RolloutCommands::Pause { version } => {
                admin.set_rollout(&version, 0).await?;
                println!("⏸️  Rollout for {} paused", version);
            }

            RolloutCommands::Resume { version } => {
                admin.set_rollout(&version, 50).await?;
                println!("▶️  Rollout for {} resumed at 50%", version);
            }

            RolloutCommands::Complete { version } => {
                admin.set_rollout(&version, 100).await?;
                println!("✅ Rollout for {} completed (100%)", version);
            }
        },

        Commands::Stats { version } => {
            let stats = admin.get_stats(version.as_deref()).await?;

            println!("Statistics");
            println!("==========");
            println!("{:#?}", stats);
        }

        Commands::Feature(cmd) => match cmd {
            FeatureCommands::Create {
                name,
                rollout,
                target_groups,
            } => {
                let req = FeatureFlagRequest {
                    name,
                    enabled: false,
                    rollout_percentage: rollout,
                    target_groups,
                };

                admin.create_feature_flag(req).await?;
            }

            _ => {
                println!("Feature flag commands not yet fully implemented");
            }
        },

        Commands::AbTest(_) => {
            println!("A/B test commands not yet fully implemented");
        }
    }

    Ok(())
}
