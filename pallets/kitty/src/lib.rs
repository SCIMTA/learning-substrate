#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_support::dispatch::fmt;
use frame_support::inherent::Vec;
use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::ArithmeticError;
use frame_support::storage::bounded_vec::BoundedVec;
use frame_support::traits::{Randomness, UnixTime};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
pub type DNA = Vec<u8>;
const MAX_BOUND: u32 = 10;
pub type KittiesBounce = BoundedVec<DNA, ConstU32<MAX_BOUND>>;

#[frame_support::pallet]
pub mod pallet {

	pub use super::*;

	#[derive(Clone, Encode, Decode, PartialEq, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		dna: DNA,
		price: u32,
		owner: T::AccountId,
		gender: Gender,
		created_date: u64,
	}

	#[derive(Clone, Encode, Decode, PartialEq, Copy, TypeInfo, MaxEncodedLen, fmt::Debug)]
	pub enum Gender {
		Male,
		Female,
	}
	impl Default for Gender {
		fn default() -> Self {
			Gender::Male
		}
	}

	impl<T: Config> fmt::Debug for Kitty<T> {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			f.debug_struct("Kitty")
				.field("dna", &self.dna)
				.field("price", &self.price)
				.field("owner", &self.owner)
				.field("gender", &self.gender)
				.field("created_date", &self.created_date)
				.finish()
		}
	}
	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type TimeProvider: UnixTime;
		type Rand: Randomness<Self::Hash, Self::BlockNumber>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn balance)]
	pub(super) type KittiesOwned<T: Config> =
		StorageMap<_, Blake2_128, T::AccountId, KittiesBounce, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn kitty_info)]
	pub(super) type Kitties<T: Config> = StorageMap<_, Blake2_128, DNA, Kitty<T>>;

	#[pallet::storage]
	#[pallet::getter(fn total_kitty)]
	pub type KittyId<T> = StorageValue<_, u32, ValueQuery, ConstU32<0>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created { kitty: DNA, owner: T::AccountId },
		Transferred { from: T::AccountId, to: T::AccountId, kitty: DNA },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		DuplicateKitty,
		TooManyOwned,
		NoKitty,
		NotOwner,
		TransferToSelf,
		OutOfBound,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(46_228_000)]
		pub fn mint(origin: OriginFor<T>, price: u32) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let start = T::TimeProvider::now();
			let nonce = KittyId::<T>::get();
			let ran = T::Rand::random(&(nonce as u128).encode());
			let dna = ran.0.encode();
			let gender = Self::gen_gender(&dna)?;
			let kitty = Kitty::<T> {
				dna: dna.clone(),
				price,
				gender,
				owner: owner.clone(),
				created_date: start.as_secs(),
			};

			// Check if the kitty does not already exist in our storage map
			ensure!(!Kitties::<T>::contains_key(&kitty.dna), Error::<T>::DuplicateKitty);

			// Performs this operation first as it may fail
			let current_id = KittyId::<T>::get();
			let next_id = current_id.checked_add(1).ok_or(ArithmeticError::Overflow)?;

			// Append kitty to KittiesOwned
			if KittiesOwned::<T>::try_append(&owner, kitty.dna.clone()).is_err() {
				return Err(Error::<T>::OutOfBound.into());
			}
			// Write new kitty to storage
			Kitties::<T>::insert(kitty.dna.clone(), kitty);
			KittyId::<T>::put(next_id);

			// Deposit our "Created" event.
			Self::deposit_event(Event::Created { kitty: dna, owner: owner.clone() });
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn transfer(origin: OriginFor<T>, dna: DNA, to: T::AccountId) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let mut kitty = Kitties::<T>::get(&dna).ok_or(Error::<T>::NoKitty)?;
			ensure!(kitty.owner == from, Error::<T>::NotOwner);
			ensure!(from != to, Error::<T>::TransferToSelf);

			let mut from_owned = KittiesOwned::<T>::get(&from);

			// Remove kitty from list of owned kitties.
			if let Some(ind) = from_owned.iter().position(|ids| *ids == dna) {
				from_owned.swap_remove(ind);
			} else {
				return Err(Error::<T>::NoKitty.into());
			}

			let mut to_owned = KittiesOwned::<T>::get(&to);
			if to_owned.try_push(dna.clone()).is_err() {
				return Err(Error::<T>::OutOfBound.into());
			}
			kitty.owner = to.clone();

			// Write updates to storage
			Kitties::<T>::insert(&dna, kitty);
			KittiesOwned::<T>::insert(&to, to_owned);
			KittiesOwned::<T>::insert(&from, from_owned);

			Self::deposit_event(Event::Transferred { from, to, kitty: dna });

			Ok(())
			// Return a successful DispatchResultWithPostInfo
		}
	}
}

impl<T> Pallet<T> {
	fn gen_gender(dna: &DNA) -> Result<Gender, Error<T>> {
		let mut res = Gender::Male;
		if dna.len() % 2 == 0 {
			res = Gender::Female;
		}
		Ok(res)
	}
}
