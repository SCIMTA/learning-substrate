#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

use frame_support::inherent::Vec;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use frame_support::traits::Currency;
use frame_support::traits::Randomness;
use frame_support::traits::UnixTime;

use sp_core::hash::H256;
const MAX_BOUND: u32 = 10;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {

	pub use super::*;

	pub type DNA = Vec<u8>;
	pub type KittyBounce = BoundedVec<DNA, ConstU32<MAX_BOUND>>;
	#[derive(TypeInfo, Default, Encode, Decode, Clone, RuntimeDebug)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		dna: DNA,
		owner: T::AccountId,
		price: BalanceOf<T>,
		gender: Gender,
		created_date: u64,
	}

	pub type Id = u32;

	#[derive(TypeInfo, Encode, Decode, Debug, Clone)]
	pub enum Gender {
		Male,
		Female,
	}
	impl Default for Gender {
		fn default() -> Self {
			Gender::Male
		}
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId>;
		type Time: UnixTime;
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn kitty_id)]
	pub type KittyId<T> = StorageValue<_, Id, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn kitty_info)]
	pub(super) type KittyInfo<T: Config> =
		StorageMap<_, Blake2_128Concat, DNA, Kitty<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn kitty)]
	pub(super) type KittyMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, KittyBounce, OptionQuery>;
	// pub(super) type SomeMap<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		KittyCreated(Vec<u8>, u32),
		KittyTransfered(T::AccountId, T::AccountId, DNA),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NotOwner,
		NotExist,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		OutOfBound,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub genesis_kitties: Vec<Vec<u8>>,
		pub owner: Option<T::AccountId>,
		pub current_time: u64,
	}
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig {
				genesis_kitties: Vec::new(),
				owner: Default::default(),
				current_time: 0,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for item in self.genesis_kitties.iter() {
				let kitty = Kitty::<T> {
					dna: H256::random().as_bytes().to_vec(),
					price: 1000u32.into(),
					owner: self.owner.clone().unwrap(),
					gender: Gender::Female,
					created_date: self.current_time,
				};
				KittyInfo::<T>::insert(item, kitty);
				let mut current_id = <KittyId<T>>::get();
				current_id += 1;
				<KittyId<T>>::put(current_id);
				if KittyMap::<T>::try_append(self.owner.clone().unwrap(), item).is_err() {
					return ();
				}
			}
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn create_kitty(origin: OriginFor<T>, price: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			log::info!("total balance: {:?}", T::Currency::total_balance(&who));
			// let dna = Self::gen_dna(seed)?;

			// randomness
			let created_date: u64 = T::Time::now().as_secs();
			let nonce = created_date as u8;
			let (randomValue, _) = T::Randomness::random(&[nonce]);

			let dna: &DNA = &randomValue.encode();
			let gender = Self::gen_gender(&dna)?;

			let kitty = Kitty {
				dna: dna.clone(),
				owner: who.clone(),
				price: 0u32.into(),
				gender,
				created_date,
			};

			let mut current_id = <KittyId<T>>::get();
			<KittyInfo<T>>::insert(&dna, kitty.clone());
			// let mut kitty_vec = <KittyMap<T>>::get(who.clone()).unwrap_or(Vec::new());
			// kitty_vec.push(kitty.dna);
			// <KittyMap<T>>::insert(who.clone(), kitty_vec);
			if KittyMap::<T>::try_append(&who, dna.clone()).is_err() {
				return Err(Error::<T>::OutOfBound.into());
			}

			current_id += 1;
			<KittyId<T>>::put(current_id);

			Self::deposit_event(Event::KittyCreated(dna.to_vec(), price));

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1, 1))]
		pub fn transfer(origin: OriginFor<T>, to: T::AccountId, dna: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let dna: &DNA = &dna;

			ensure!(<KittyInfo<T>>::contains_key(&dna), Error::<T>::NotExist);
			let mut kitty = <KittyInfo<T>>::get(&dna).unwrap();
			ensure!(kitty.owner == who, Error::<T>::NotOwner);
			ensure!(<KittyMap<T>>::get(who.clone()).unwrap().contains(&dna), Error::<T>::NotOwner);

			let mut kitty_vec = <KittyMap<T>>::get(who.clone()).unwrap();

			// remove old owner
			let mut index = 0;
			for (count, v) in kitty_vec.iter().enumerate() {
				// let s1: HashSet<_> = v.iter().copied().collect();
				// let s2: HashSet<_> = dna.to_vec().iter().copied().collect();
				// let diff: Vec<_> = s1.difference(&s2).collect();
				// if diff.len() > 0 {
				// 	break;
				// }
				if Self::check_equal_vec(v, dna)? {
					break;
				}
				index += 1;
			}
			<KittyMap<T>>::get(who.clone()).unwrap().remove(index);

			// add new owner
			kitty.owner = to.clone();
			// <KittyInfo<T>>::insert(&dna, kitty.clone());
			// let mut kitty_vec_to = <KittyMap<T>>::get(to.clone());
			// kitty_vec_to.push(kitty.dna);
			// <KittyMap<T>>::insert(to.clone(), kitty_vec_to);
			if KittyMap::<T>::try_append(&to, dna.clone()).is_err() {
				return Err(Error::<T>::OutOfBound.into());
			}

			Self::deposit_event(Event::KittyTransfered(who, to, dna.clone()));

			Ok(())
		}
	}
}

impl<T> Pallet<T> {
	// fn gen_dna(seed: u32) -> Result<Vec<u8>, Error<T>> {
	// 	// let mut rng = rand::thread_rng();
	// 	// let range = Uniform::new(0, 20 + seed as u8);
	// 	// generate your random seed

	// 	let a: [u8; 3] = [1, 2, 3];
	// 	let (random_seed, _) = Randomness::random(&a);
	// 	let random_number = <u32>::decode(&mut random_seed.as_ref())
	// 		.expect("secure hashes should always be bigger than u32; qed");

	// 	Ok(())
	// }

	fn gen_gender(dna: &Vec<u8>) -> Result<Gender, Error<T>> {
		let mut res = Gender::Female;
		if dna.len() % 2 == 0 {
			res = Gender::Male;
		}

		Ok(res)
	}

	fn check_equal_vec(arr1: &Vec<u8>, arr2: &Vec<u8>) -> Result<bool, Error<T>> {
		Ok(arr1.iter().eq(arr2.iter()))
	}
}
