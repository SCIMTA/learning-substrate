#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

use frame_support::inherent::Vec;
use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::SaturatedConversion;
use frame_support::traits::Time;
use frame_system::pallet_prelude::*;
use pallet_token::TokenInterface;
use sp_core::Bytes;

// Time config
pub type BlockNumber = u32;
pub const MILLISECS_PER_BLOCK: u64 = 6000;
// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;
// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;

pub const minProposalDebatePeriod: BlockNumber = 2 * WEEKS;
pub const quorumHalvingPeriod: BlockNumber = 25 * WEEKS;
pub const executeProposalPeriod: BlockNumber = 10 * DAYS;
pub const preSupportTime: BlockNumber = 2 * DAYS;
pub const maxDepositDivisor: u128 = 100;

//impl clone for storagemap
// use frame_support::storage::Key;
use codec::FullCodec;
use frame_support::traits::StorageInstance;
use frame_support::StorageHasher;

// impl Clone for StorageMap<Prefix, Hasher, Key, Value> {
// 	fn clone(&self) -> Self {
// 		let mut new_map = Self::new();
// 		for (k, v) in self.iter() {
// 			new_map.insert(k, v);
// 		}
// 		new_map
// }

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	// A proposal with `newCurator == false` represents a transaction
	// to be issued by this DAO
	// A proposal with `newCurator == true` represents a DAO split

	#[derive(Encode, Decode)]
	struct Proposal<T: Config, U> {
		// The address where the `amount` will go to if the proposal is accepted
		recipient: T::AccountId,
		// The amount to transfer to `recipient` if the proposal is accepted.
		amount: u128,
		// A plain text description of the proposal
		description: Vec<u8>,
		// A unix timestamp, denoting the end of the voting period
		votingDeadline: T::BlockNumber,
		// True if the proposal's votes have yet to be counted, otherwise False
		open: bool,
		// True if quorum has been reached, the votes have been counted, and
		// the majority said yes
		proposalPassed: bool,
		// A hash to check validity of a proposal
		//TODO: change to hash
		proposalHash: Bytes,
		// Deposit in wei the creator added when submitting their proposal. It
		// is taken from the msg.value of a newProposal call.
		proposalDeposit: u128,
		// True if this proposal is to assign a new Curator
		newCurator: bool,
		// true if more tokens are in favour of the proposal than opposed to it at
		// least `preSupportTime` before the voting deadline
		preSupport: bool,
		// Number of Tokens in favor of the proposal
		yea: u128,
		// Number of Tokens opposed to the proposal
		nay: u128,
		// Simple mapping to check if a shareholder has voted for it
		votedYes: StorageMap<U, Blake2_128Concat, T::AccountId, bool, OptionQuery>,
		// // Simple mapping to check if a shareholder has voted against it
		votedNo: StorageMap<U, Blake2_128Concat, T::AccountId, bool, OptionQuery>,
		// Address of the shareholder who created the proposal
		creator: T::AccountId,
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Time: Time;
		type TokenInterface: TokenInterface<who = Self::AccountId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn proposalsId)]
	pub type proposalsId<T> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn proposals)]
	pub type proposals<T: Config> =
		StorageMap<_, Blake2_128Concat, u128, Proposal<T, U>, OptionQuery>;

	// The quorum needed for each proposal is partially calculated by
	// totalSupply / minQuorumDivisor
	#[pallet::storage]
	#[pallet::getter(fn minQuorumDivisor)]
	pub(super) type minQuorumDivisor<T> = StorageValue<_, u128, ValueQuery>;

	// The unix time of the last time quorum was reached on a proposal
	#[pallet::storage]
	#[pallet::getter(fn lastTimeMinQuorumMet)]
	pub(super) type lastTimeMinQuorumMet<T> = StorageValue<_, BlockNumber, ValueQuery>;

	// Address of the curator of this DAO
	#[pallet::storage]
	#[pallet::getter(fn curator)]
	pub(super) type curator<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	// The whitelist: List of addresses the DAO is allowed to send ether to
	#[pallet::storage]
	#[pallet::getter(fn allowedRecipients)]
	pub(super) type allowedRecipients<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, bool, OptionQuery>;

	// Map of addresses blocked during a vote (not allowed to transfer DAO
	// tokens). The address points to the proposal ID.
	#[pallet::storage]
	#[pallet::getter(fn _blocked)]
	pub(super) type _blocked<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u128, OptionQuery>;

	// Map of addresses and proposal voted on by this address
	#[pallet::storage]
	#[pallet::getter(fn votingRegister)]
	pub(super) type votingRegister<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Vec<u128>, OptionQuery>;

	// The minimum deposit (in wei) required to submit any proposal that is not
	// requesting a new Curator (no deposit is required for splits)
	#[pallet::storage]
	#[pallet::getter(fn proposalDeposit)]
	pub(super) type proposalDeposit<T> = StorageValue<_, u128, ValueQuery>;

	// the accumulated sum of all current proposal deposits
	#[pallet::storage]
	#[pallet::getter(fn sumOfProposalDeposits)]
	pub(super) type sumOfProposalDeposits<T> = StorageValue<_, u128, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// Util events
		CallBalances(u128, T::AccountId),
		CallCurrentTime(u128),

		// Proposal
		ProposalAdded(u128, T::AccountId, u128, Vec<u8>),
		Voted(u128, bool, T::AccountId),
		ProposalTallied(u128, bool, u128, u128),
		AllowedRecipientChanged(T::AccountId, bool),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().reads(1))]
		pub fn call_balances(origin: OriginFor<T>, _who: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let balance = T::TokenInterface::_balanceOf(_who.clone());
			Self::deposit_event(Event::CallBalances(balance, _who));
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads(1))]
		pub fn get_current_time(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let time = T::Time::now();
			Self::deposit_event(Event::CallCurrentTime(time.saturated_into::<u64>().into()));
			Ok(())
		}

		#[pallet::weight(10_000 + 500_000_000)]
		pub fn expensive_or_cheap(origin: OriginFor<T>, input: u64) -> DispatchResultWithPostInfo {
			Ok(Some(10_000).into())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads(1))]
		pub fn DAO(
			origin: OriginFor<T>,
			_currator: T::AccountId,
			_proposalDeposit: u128,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let time = T::Time::now();
			let _timeCurrent: u32 = time.saturated_into::<u32>();

			<curator<T>>::put(_currator);
			<proposalDeposit<T>>::put(_proposalDeposit);
			<lastTimeMinQuorumMet<T>>::put(_timeCurrent);
			<minQuorumDivisor<T>>::put(7);

			// Add empty proposal to ignore 0
			// <proposals<T>>::put(Vec::new());

			// let balance = T::TokenInterface::_balanceOf(_who.clone());
			// Self::deposit_event(Event::CallBalances(balance, _who));
			Ok(())
		}
	}
}
