
//! Autogenerated weights for pallet_rewards
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-03-06, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `nazar-pc`, CPU: `13th Gen Intel(R) Core(TM) i9-13900K`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// target/release/subspace-node
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_rewards
// --extrinsic=*
// --heap-pages=4096
// --output=crates/pallet-rewards/src/weights.rs
// --template=frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::ParityDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for pallet_rewards.
pub trait WeightInfo {
	fn update_issuance_params(p: u32, v: u32, ) -> Weight;
}

/// Weights for pallet_rewards using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: `Rewards::ProposerSubsidyPoints` (r:0 w:1)
	/// Proof: `Rewards::ProposerSubsidyPoints` (`max_values`: Some(1), `max_size`: Some(401), added: 896, mode: `MaxEncodedLen`)
	/// Storage: `Rewards::VoterSubsidyPoints` (r:0 w:1)
	/// Proof: `Rewards::VoterSubsidyPoints` (`max_values`: Some(1), `max_size`: Some(401), added: 896, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[0, 20]`.
	/// The range of component `v` is `[0, 20]`.
	fn update_issuance_params(p: u32, v: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_978_000 picoseconds.
		Weight::from_parts(5_191_890, 0)
			// Standard Error: 400
			.saturating_add(Weight::from_parts(4_397, 0).saturating_mul(p.into()))
			// Standard Error: 400
			.saturating_add(Weight::from_parts(4_865, 0).saturating_mul(v.into()))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: `Rewards::ProposerSubsidyPoints` (r:0 w:1)
	/// Proof: `Rewards::ProposerSubsidyPoints` (`max_values`: Some(1), `max_size`: Some(401), added: 896, mode: `MaxEncodedLen`)
	/// Storage: `Rewards::VoterSubsidyPoints` (r:0 w:1)
	/// Proof: `Rewards::VoterSubsidyPoints` (`max_values`: Some(1), `max_size`: Some(401), added: 896, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[0, 20]`.
	/// The range of component `v` is `[0, 20]`.
	fn update_issuance_params(p: u32, v: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_978_000 picoseconds.
		Weight::from_parts(5_191_890, 0)
			// Standard Error: 400
			.saturating_add(Weight::from_parts(4_397, 0).saturating_mul(p.into()))
			// Standard Error: 400
			.saturating_add(Weight::from_parts(4_865, 0).saturating_mul(v.into()))
			.saturating_add(ParityDbWeight::get().writes(2_u64))
	}
}