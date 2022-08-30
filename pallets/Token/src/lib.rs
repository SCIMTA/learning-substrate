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

use frame_support::dispatch::DispatchResultWithPostInfo;
use frame_support::inherent::Vec;
use frame_support::pallet_prelude::*;
use frame_support::weights::PostDispatchInfo;
use frame_support::BoundedVec;
use frame_system::pallet_prelude::*;

use frame_support::traits::Currency;
use frame_support::traits::Randomness;
use frame_support::traits::UnixTime;

#[frame_support::pallet]
pub mod pallet {
	pub use super::*;

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Default, Encode, Decode, Clone, RuntimeDebug, scale_info::TypeInfo)]
	pub struct TokenInfoStruct {
		pub name: Vec<u8>,
		pub symbol: Vec<u8>,
		pub decimals: u8,
		pub total_supply: u128,
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
		type Currency: Currency<Self::AccountId>;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub name: Vec<u8>,
		pub symbol: Vec<u8>,
		pub decimals: u8,
		pub total_supply: u128,
		pub owner: Option<T::AccountId>,
	}
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig {
				name: Vec::new(),
				symbol: Vec::new(),
				decimals: 0,
				total_supply: 0,
				owner: Default::default(),
			}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			let owner = self.owner.clone().unwrap();
			let total_supply = self.total_supply;
			let symbol = self.symbol.clone();
			let name = self.name.clone();
			let decimals = self.decimals;
			<_name<T>>::put(name);
			<symbol<T>>::put(symbol);
			<decimals<T>>::put(decimals);
			<totalSupply<T>>::put(total_supply);
			<owner<T>>::put(owner.clone());
			<balances<T>>::insert(owner, total_supply);
		}
	}

	// The pallet's runtime storage items
	#[pallet::storage]
	#[pallet::getter(fn _name)]
	pub(super) type _name<T: Config> = StorageValue<_, Vec<u8>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn symbol)]
	pub(super) type symbol<T: Config> = StorageValue<_, Vec<u8>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn decimals)]
	pub(super) type decimals<T: Config> = StorageValue<_, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn totalSupply)]
	pub(super) type totalSupply<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn owner)]
	pub(super) type owner<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn balances)]
	pub type balances<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u128, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn allowed)]
	pub(super) type allowed<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		T::AccountId,
		u128,
		OptionQuery,
	>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		Transfered(T::AccountId, T::AccountId, u128),
		TransferedFrom(T::AccountId, T::AccountId, T::AccountId, u128),
		Approval(T::AccountId, T::AccountId, u128),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		InsufficientFunds,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn approve(origin: OriginFor<T>, spender: T::AccountId, value: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			<allowed<T>>::insert(sender.clone(), spender.clone(), value);

			Self::deposit_event(Event::Approval(sender, spender, value));

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1, 1))]
		pub fn transfer(origin: OriginFor<T>, to: T::AccountId, amount: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let sender_balance = <balances<T>>::get(&sender).unwrap_or_default();
			let new_balance =
				sender_balance.checked_sub(amount).ok_or(Error::<T>::StorageOverflow)?;
			let receiver_balance = <balances<T>>::get(&to).unwrap_or_default();
			let new_receiver_balance =
				receiver_balance.checked_add(amount).ok_or(Error::<T>::StorageOverflow)?;
			<balances<T>>::insert(&sender, new_balance);
			<balances<T>>::insert(&to, new_receiver_balance);

			Self::deposit_event(Event::Transfered(sender, to, amount));

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1, 1))]
		pub fn transferFrom(
			origin: OriginFor<T>,
			from: T::AccountId,
			to: T::AccountId,
			amount: u128,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let sender_allowed_balance =
				<allowed<T>>::get(&from.clone(), sender.clone()).unwrap_or_default();
			let new_allowed_balance =
				sender_allowed_balance.checked_sub(amount).ok_or(Error::<T>::StorageOverflow)?;
			let from_balance = <balances<T>>::get(&from).unwrap_or_default();
			let new_from_balance =
				from_balance.checked_sub(amount).ok_or(Error::<T>::StorageOverflow)?;
			let receiver_balance = <balances<T>>::get(&to).unwrap_or_default();
			let new_receiver_balance =
				receiver_balance.checked_add(amount).ok_or(Error::<T>::StorageOverflow)?;
			<allowed<T>>::insert(&from.clone(), sender.clone(), new_allowed_balance);
			<balances<T>>::insert(&from, new_from_balance);
			<balances<T>>::insert(&to, new_receiver_balance);

			Self::deposit_event(Event::TransferedFrom(sender, from, to, amount));

			Ok(())
		}
	}
}
