use crate::identity::Identity;
use crate::rpc_client::RpcClient;
use crate::single_disk_farm::SingleDiskSemaphore;
use anyhow::anyhow;
use derive_more::{Display, From};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::num::NonZeroU16;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::SystemTime;
use std::{fs, io};
use subspace_core_primitives::PublicKey;
use subspace_networking::Node;
use subspace_rpc_primitives::FarmerProtocolInfo;
use subspace_solving::SubspaceCodec;
use tracing::{error, Instrument, Span};
use ulid::Ulid;

/// An identifier for single disk plot, can be used for in logs, thread names, etc.
#[derive(
    Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Display, From,
)]
#[serde(untagged)]
pub enum SingleDiskPlotId {
    /// Plot ID
    Ulid(Ulid),
}

#[allow(clippy::new_without_default)]
impl SingleDiskPlotId {
    /// Creates new ID
    pub fn new() -> Self {
        Self::Ulid(Ulid::new())
    }
}

/// Important information about the contents of the `SingleDiskPlot`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SingleDiskPlotInfo {
    /// V0 of the info
    #[serde(rename_all = "camelCase")]
    V0 {
        /// ID of the plot
        id: SingleDiskPlotId,
        /// Genesis hash of the chain used for plot creation
        #[serde(with = "hex::serde")]
        genesis_hash: [u8; 32],
        /// Public key of identity used for plot creation
        public_key: PublicKey,
        /// First sector index in this plot
        ///
        /// Multiple plots can reuse the same identity, but they have to use different ranges for
        /// sector indexes or else they'll essentially plot the same data and will not result in
        /// increased probability of winning the reward.
        first_sector_index: u64,
        /// How much space in bytes is allocated for plot in this plot (metadata space is not
        /// included)
        allocated_plotting_space: u64,
    },
}

impl SingleDiskPlotInfo {
    const FILE_NAME: &'static str = "single_disk_plot.json";

    pub fn new(
        id: SingleDiskPlotId,
        genesis_hash: [u8; 32],
        public_key: PublicKey,
        first_sector_index: u64,
        allocated_plotting_space: u64,
    ) -> Self {
        Self::V0 {
            id,
            genesis_hash,
            public_key,
            first_sector_index,
            allocated_plotting_space,
        }
    }

    /// Load `SingleDiskPlot` from path is supposed to be stored, `None` means no info file was
    /// found, happens during first start.
    pub fn load_from(path: &Path) -> io::Result<Option<Self>> {
        let bytes = match fs::read(path.join(Self::FILE_NAME)) {
            Ok(bytes) => bytes,
            Err(error) => {
                return if error.kind() == io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(error)
                };
            }
        };

        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    /// Store `SingleDiskPlot` info to path so it can be loaded again upon restart.
    pub fn store_to(&self, directory: &Path) -> io::Result<()> {
        fs::write(
            directory.join(Self::FILE_NAME),
            serde_json::to_vec(self).expect("Info serialization never fails; qed"),
        )
    }

    // ID of the plot
    pub fn id(&self) -> &SingleDiskPlotId {
        let Self::V0 { id, .. } = self;
        id
    }

    // Genesis hash of the chain used for plot creation
    pub fn genesis_hash(&self) -> &[u8; 32] {
        let Self::V0 { genesis_hash, .. } = self;
        genesis_hash
    }

    // Public key of identity used for plot creation
    pub fn public_key(&self) -> &PublicKey {
        let Self::V0 { public_key, .. } = self;
        public_key
    }

    /// First sector index in this plot
    ///
    /// Multiple plots can reuse the same identity, but they have to use different ranges for
    /// sector indexes or else they'll essentially plot the same data and will not result in
    /// increased probability of winning the reward.
    pub fn first_sector_index(&self) -> u64 {
        let Self::V0 {
            first_sector_index, ..
        } = self;
        *first_sector_index
    }

    /// How much space in bytes is allocated for plot in this plot (metadata space is not included)
    pub fn allocated_plotting_space(&self) -> u64 {
        let Self::V0 {
            allocated_plotting_space,
            ..
        } = self;
        *allocated_plotting_space
    }
}

/// Summary of single disk plot for presentational purposes
pub enum SingleDiskPlotSummary {
    /// Plot was found and read successfully
    Found {
        // ID of the plot
        id: SingleDiskPlotId,
        // Genesis hash of the chain used for plot creation
        genesis_hash: [u8; 32],
        // Public key of identity used for plot creation
        public_key: PublicKey,
        /// First sector index in this plot
        first_sector_index: u64,
        // How much space in bytes can plot use for plot (metadata space is not included)
        allocated_plotting_space: u64,
        /// Path to directory where plot is stored.
        directory: PathBuf,
    },
    /// Plot was not found
    NotFound {
        /// Path to directory where plot is stored.
        directory: PathBuf,
    },
    /// Failed to open plot
    Error {
        /// Path to directory where plot is stored.
        directory: PathBuf,
        /// Error itself
        error: io::Error,
    },
}

/// Options used to open single dis plot
pub struct SingleDiskPlotOptions<RC> {
    /// Path to directory where plot are stored.
    pub directory: PathBuf,
    /// How much space in bytes can farm use for plot
    pub allocated_plotting_space: u64,
    /// Identity associated with plot
    pub identity: Identity,
    /// Networking instance for external communication with DSN
    pub node: Node,
    /// RPC client connected to Subspace node
    pub rpc_client: RC,
    /// Address where farming rewards should go
    pub reward_address: PublicKey,
    /// Information about protocol necessary for farmer
    pub farmer_protocol_info: FarmerProtocolInfo,
}

/// Single disk plot abstraction is a container for everything necessary to plot/farm with a single
/// disk plot.
#[must_use = "Plot does not function properly unless run() method is called"]
pub struct SingleDiskPlot {
    id: SingleDiskPlotId,
    span: Span,
    tasks: FuturesUnordered<Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>>,
}

impl SingleDiskPlot {
    /// Create new single disk plot instance
    pub fn new<RC>(options: SingleDiskPlotOptions<RC>) -> anyhow::Result<Self>
    where
        RC: RpcClient,
    {
        let SingleDiskPlotOptions {
            directory,
            allocated_plotting_space,
            identity,
            // TODO: Use this or remove
            node: _,
            farmer_protocol_info,
            // TODO: Use this or remove
            rpc_client: _,
            // TODO: Use this or remove
            reward_address: _,
        } = options;

        fs::create_dir_all(&directory)?;

        // TODO: Parametrize concurrency, much higher default due to SSD focus
        // TODO: Use this or remove
        let _single_disk_semaphore =
            SingleDiskSemaphore::new(NonZeroU16::new(10).expect("Not a zero; qed"));

        let public_key = identity.public_key().to_bytes().into();

        let single_disk_plot_info = match SingleDiskPlotInfo::load_from(&directory)? {
            Some(single_disk_plot_info) => {
                if allocated_plotting_space != single_disk_plot_info.allocated_plotting_space() {
                    error!(
                        id = %single_disk_plot_info.id(),
                        "Usable plotting space {} is different from {} when farm was created, \
                        resizing isn't supported yet",
                        allocated_plotting_space,
                        single_disk_plot_info.allocated_plotting_space(),
                    );

                    return Err(anyhow!("Can't resize farm after creation"));
                }

                if &farmer_protocol_info.genesis_hash != single_disk_plot_info.genesis_hash() {
                    error!(
                        id = %single_disk_plot_info.id(),
                        "Genesis hash {} is different from {} when farm was created, is is not \
                        possible to use farm on a different chain",
                        hex::encode(farmer_protocol_info.genesis_hash),
                        hex::encode(single_disk_plot_info.genesis_hash()),
                    );

                    return Err(anyhow!("Wrong chain (genesis hash)"));
                }

                if &public_key != single_disk_plot_info.public_key() {
                    error!(
                        id = %single_disk_plot_info.id(),
                        "Public key {} is different from {} when farm was created, something \
                        went wrong, likely due to manual edits",
                        hex::encode(&public_key),
                        hex::encode(single_disk_plot_info.public_key()),
                    );

                    return Err(anyhow!("Public key in identity doesn't match metadata"));
                }

                single_disk_plot_info
            }
            None => {
                // TODO: Global generator that makes sure to avoid returning the same sector index for multiple disks
                let first_sector_index = SystemTime::UNIX_EPOCH
                    .elapsed()
                    .expect("Unix epoch is always in the past; qed")
                    .as_secs()
                    .wrapping_mul(u64::from(u32::MAX));

                let single_disk_plot_info = SingleDiskPlotInfo::new(
                    SingleDiskPlotId::new(),
                    farmer_protocol_info.genesis_hash,
                    public_key,
                    first_sector_index,
                    allocated_plotting_space,
                );

                single_disk_plot_info.store_to(&directory)?;

                single_disk_plot_info
            }
        };

        // TODO: Use this or remove
        let _codec = SubspaceCodec::new_with_gpu(public_key.as_ref());

        let tasks =
            FuturesUnordered::<Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>>::new();

        let farm = Self {
            id: *single_disk_plot_info.id(),
            span: Span::current(),
            tasks,
        };

        Ok(farm)
    }

    /// Collect summary of single disk plot for presentational purposes
    pub fn collect_summary(directory: PathBuf) -> SingleDiskPlotSummary {
        let single_disk_plot_info = match SingleDiskPlotInfo::load_from(&directory) {
            Ok(Some(single_disk_plot_info)) => single_disk_plot_info,
            Ok(None) => {
                return SingleDiskPlotSummary::NotFound { directory };
            }
            Err(error) => {
                return SingleDiskPlotSummary::Error { directory, error };
            }
        };

        return SingleDiskPlotSummary::Found {
            id: *single_disk_plot_info.id(),
            genesis_hash: *single_disk_plot_info.genesis_hash(),
            public_key: *single_disk_plot_info.public_key(),
            first_sector_index: single_disk_plot_info.first_sector_index(),
            allocated_plotting_space: single_disk_plot_info.allocated_plotting_space(),
            directory,
        };
    }

    /// ID of this farm
    pub fn id(&self) -> &SingleDiskPlotId {
        &self.id
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        while let Some(result) = self.tasks.next().instrument(self.span.clone()).await {
            result?;
        }

        Ok(())
    }
}
