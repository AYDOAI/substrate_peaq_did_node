//! # Template Pallet
//!
//! A pallet with minimal functionality to help developers understand the essential components of
//! writing a FRAME pallet. It is typically used in beginner tutorials or in Substrate template
//! nodes as a starting point for creating a new pallet and **not meant to be used in production**.
//!
//! ## Overview
//!
//! This template pallet contains basic examples of:
//! - declaring a storage item that stores a single `u32` value
//! - declaring and using events
//! - declaring and using errors
//! - a dispatchable function that allows a user to set a new value to storage and emits an event
//!   upon success
//! - another dispatchable function that causes a custom error to be thrown
//!
//! Each pallet section is annotated with an attribute using the `#[pallet::...]` procedural macro.
//! This macro generates the necessary code for a pallet to be aggregated into a FRAME runtime.
//!
//! Learn more about FRAME macros [here](https://docs.substrate.io/reference/frame-macros/).
//!
//! ### Pallet Sections
//!
//! The pallet sections in this template are:
//!
//! - A **configuration trait** that defines the types and parameters which the pallet depends on
//!   (denoted by the `#[pallet::config]` attribute). See: [`Config`].
//! - A **means to store pallet-specific data** (denoted by the `#[pallet::storage]` attribute).
//!   See: [`storage_types`].
//! - A **declaration of the events** this pallet emits (denoted by the `#[pallet::event]`
//!   attribute). See: [`Event`].
//! - A **declaration of the errors** that this pallet can throw (denoted by the `#[pallet::error]`
//!   attribute). See: [`Error`].
//! - A **set of dispatchable functions** that define the pallet's functionality (denoted by the
//!   `#[pallet::call]` attribute). See: [`dispatchables`].
//!
//! Run `cargo doc --package pallet-template --open` to view this pallet's documentation.

// We make sure this pallet uses `no_std` for compiling to Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod did;
pub mod structs;
// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

pub mod weightinfo;
pub mod weights;
pub use weightinfo::WeightInfo;

// FRAME pallets require their own "mock runtimes" to be able to run unit tests. This module
// contains a mock runtime specific for testing this pallet's functionality.
#[cfg(test)]
mod mock;

// This module contains the unit tests for this pallet.
// Learn about pallet unit testing here: https://docs.substrate.io/test/unit-testing/
#[cfg(test)]
mod tests;

// Every callable function or "dispatchable" a pallet exposes must have weight values that correctly
// estimate a dispatchable's execution time. The benchmarking module is used to calculate weights
// for each dispatchable and generates this pallet's weight.rs file. Learn more about benchmarking here: https://docs.substrate.io/test/benchmark/
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[frame_support::pallet]
pub mod pallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;

	use crate::{
		did::{DidError, *},
		structs::*,
	};

	use frame_support::pallet_prelude::*;
	pub use frame_support::traits::Time as MomentTime;
	use frame_system::pallet_prelude::*;

	use sp_core::blake2_256;
	use sp_runtime::traits::{Bounded, CheckedAdd};
	use sp_std::vec::Vec;

	/// The pallet's configuration trait.
	///
	/// All our types and constants a pallet depends on must be declared here.
	/// These types are defined generically and made concrete when the pallet is declared in the
	/// `runtime/src/lib.rs` file of your chain.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: WeightInfo;

		type Time: MomentTime;
	}

	/// Events that functions in this pallet can emit.
	///
	/// Events are a simple means of indicating to the outside world (such as dApps, chain explorers
	/// or other users) that some notable update in the runtime has occurred. In a FRAME pallet, the
	/// documentation for each event field and its parameters is added to a node's metadata so it
	/// can be used by external interfaces or tools.
	///
	///	The `generate_deposit` macro generates a function on `Pallet` called `deposit_event` which
	/// will convert the event type of your pallet into `RuntimeEvent` (declared in the pallet's
	/// [`Config`] trait) and deposit it using [`frame_system::Pallet::deposit_event`].
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A user has successfully set a new value.
		AttributeAdded(T::AccountId, T::AccountId, Vec<u8>, Vec<u8>, Option<BlockNumberFor<T>>),
		/// Event emitted when an attribute is read successfully
		AttributeRead(Attribute<BlockNumberFor<T>, <<T as Config>::Time as MomentTime>::Moment>),
		/// Event emitted when an attribute has been updated. [who, did_account, name, validity]
		AttributeUpdated(T::AccountId, T::AccountId, Vec<u8>, Vec<u8>, Option<BlockNumberFor<T>>),
		/// Event emitted when an attribute has been deleted. [who, did_acount name]
		AttributeRemoved(T::AccountId, T::AccountId, Vec<u8>),
	}

	/// Errors that can be returned by this pallet.
	///
	/// Errors tell users that something went wrong so it's important that their naming is
	/// informative. Similar to events, error documentation is added to a node's metadata so it's
	/// equally important that they have helpful documentation associated with them.
	///
	/// This type of runtime error can be up to 4 bytes in size should you want to return additional
	/// information.
	#[pallet::error]
	pub enum Error<T> {
		// Name is greater that 64
		AttributeNameExceedMax64,
		// Attribute already exist
		AttributeAlreadyExist,
		// Attribute creation failed
		AttributeCreationFailed,
		// Attribute creation failed
		AttributeUpdateFailed,
		// Attribute was not found
		AttributeNotFound,
		// Dispatch when trying to modify another owner did
		AttributeAuthorizationFailed,
		// Dispatch when block number is invalid
		MaxBlockNumberExceeded,
		InvalidSuppliedValue,
		ParseError,
	}

	impl<T: Config> Error<T> {
		fn dispatch_error(err: DidError) -> DispatchResult {
			match err {
				DidError::NotFound => Err(Error::<T>::AttributeNotFound.into()),
				DidError::AlreadyExist => Err(Error::<T>::AttributeAlreadyExist.into()),
				DidError::NameExceedMaxChar => Err(Error::<T>::AttributeNameExceedMax64.into()),
				DidError::FailedCreate => Err(Error::<T>::AttributeCreationFailed.into()),
				DidError::FailedUpdate => Err(Error::<T>::AttributeCreationFailed.into()),
				DidError::AuthorizationFailed =>
					Err(Error::<T>::AttributeAuthorizationFailed.into()),
				DidError::MaxBlockNumberExceeded => Err(Error::<T>::MaxBlockNumberExceeded.into()),
			}
		}
	}

	// The `Pallet` struct serves as a placeholder to implement traits, methods and dispatchables
	// (`Call`s) in this pallet.
	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn attribute_of)]
	pub(super) type AttributeStore<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		[u8; 32],
		Attribute<BlockNumberFor<T>, <<T as Config>::Time as MomentTime>::Moment>,
		ValueQuery,
	>;
	/// A storage item for this pallet.
	///
	/// In this template, we are declaring a storage item called `Something` that stores a single
	/// `u32` value. Learn more about runtime storage here: <https://docs.substrate.io/build/runtime-storage/>
	#[pallet::storage]
	#[pallet::getter(fn owner_of)]
	pub(super) type OwnerStore<T: Config> =
		StorageMap<_, Blake2_128Concat, (T::AccountId, [u8; 32]), T::AccountId>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	/// The pallet's dispatchable functions ([`Call`]s).
	///
	/// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	/// These functions materialize as "extrinsics", which are often compared to transactions.
	/// They must always return a `DispatchResult` and be annotated with a weight and call index.
	///
	/// The [`call_index`] macro is used to explicitly
	/// define an index for calls in the [`Call`] enum. This is useful for pallets that may
	/// introduce new dispatchables over time. If the order of a dispatchable changes, its index
	/// will also change which will break backwards compatibility.
	///
	/// The [`weight`] macro is used to assign a weight to each call.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a single u32 value as a parameter, writes the value
		/// to storage and emits an event.
		///
		/// It checks that the _origin_ for this call is _Signed_ and returns a dispatch
		/// error if it isn't. Learn more about origins here: <https://docs.substrate.io/build/origins/>
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::add_attribute())]
		pub fn add_attribute(
			origin: OriginFor<T>,
			did_account: T::AccountId,
			name: Vec<u8>,
			value: Vec<u8>,
			valid_for: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			// Check that an extrinsic was signed and get the signer
			// This fn returns an error if the extrinsic is not signed
			// https://docs.substrate.io/v3/runtime/origins
			let sender = ensure_signed(origin)?;

			// Verify that the name len is 64 max
			ensure!(name.len() <= 64, Error::<T>::AttributeNameExceedMax64);

			match Self::create(&sender, &did_account, &name, &value, valid_for) {
				Ok(()) => {
					Self::deposit_event(Event::AttributeAdded(
						sender,
						did_account,
						name,
						value,
						valid_for,
					));
				},
				Err(e) => return Error::<T>::dispatch_error(e),
			};

			Ok(())
		}

		/// Update an existing attribute of a DID
		/// with optional validity
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::update_attribute())]
		pub fn update_attribute(
			origin: OriginFor<T>,
			did_account: T::AccountId,
			name: Vec<u8>,
			value: Vec<u8>,
			valid_for: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			// Check that an extrinsic was signed and get the signer
			// This fn returns an error if the extrinsic is not signed
			// https://docs.substrate.io/v3/runtime/origins
			let sender = ensure_signed(origin)?;

			// Verify that the name len is 64 max
			ensure!(name.len() <= 64, Error::<T>::AttributeNameExceedMax64);

			match Self::update(&sender, &did_account, &name, &value, valid_for) {
				Ok(()) => {
					Self::deposit_event(Event::AttributeUpdated(
						sender,
						did_account,
						name,
						value,
						valid_for,
					));
				},
				Err(e) => return Error::<T>::dispatch_error(e),
			};
			Ok(())
		}

		/// Read did attribute
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::read_attribute())]
		pub fn read_attribute(
			origin: OriginFor<T>,
			did_account: T::AccountId,
			name: Vec<u8>,
		) -> DispatchResult {
			// Check that an extrinsic was signed and get the signer
			// This fn returns an error if the extrinsic is not signed
			// https://docs.substrate.io/v3/runtime/origins
			ensure_signed(origin)?;

			let attribute = Self::read(&did_account, &name);
			match attribute {
				Some(attribute) => {
					Self::deposit_event(Event::AttributeRead(attribute));
				},
				None => return Err(Error::<T>::AttributeNotFound.into()),
			}
			Ok(())
		}

		/// Delete an existing attribute of a DID
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::remove_attribute())]
		pub fn remove_attribute(
			origin: OriginFor<T>,
			did_account: T::AccountId,
			name: Vec<u8>,
		) -> DispatchResult {
			// Check that an extrinsic was signed and get the signer
			// This fn returns an error if the extrinsic is not signed
			// https://docs.substrate.io/v3/runtime/origins
			let sender = ensure_signed(origin)?;

			// Verify that the name len is 64 max
			ensure!(name.len() <= 64, Error::<T>::AttributeNameExceedMax64);

			match Self::delete(&sender, &did_account, &name) {
				Ok(()) => {
					// Get the block number from the FRAME system pallet
					Self::deposit_event(Event::AttributeRemoved(sender, did_account, name));
				},
				Err(e) => return Error::<T>::dispatch_error(e),
			};
			Ok(())
		}
	}

	impl<T: Config>
		Did<T::AccountId, BlockNumberFor<T>, <<T as Config>::Time as MomentTime>::Moment> for Pallet<T>
	{
		fn is_owner(owner: &T::AccountId, did_account: &T::AccountId) -> Result<(), DidError> {
			let id = (&owner, &did_account).using_encoded(blake2_256);

			// Check if attribute already exists
			if !<OwnerStore<T>>::contains_key((&owner, &id)) {
				return Err(DidError::AuthorizationFailed);
			}

			Ok(())
		}

		// Add new attribute to a did
		fn create(
			owner: &T::AccountId,
			did_account: &T::AccountId,
			name: &[u8],
			value: &[u8],
			valid_for: Option<BlockNumberFor<T>>,
		) -> Result<(), DidError> {
			// Generate id for integrity check
			let id = Self::get_hashed_key_for_attr(did_account, name);

			// Check if attribute already exists
			if <AttributeStore<T>>::contains_key(id) {
				return Err(DidError::AlreadyExist);
			}

			let now_timestamp = T::Time::now();

			// validate block number to prevent an overflow
			let validity = match Self::validate_block_number(valid_for) {
				Ok(validity) => validity,
				Err(e) => return Err(e),
			};

			let new_attribute = Attribute {
				name: name.to_vec(),
				value: value.to_vec(),
				validity,
				created: now_timestamp,
			};

			<AttributeStore<T>>::insert(id, new_attribute);

			// Store the owner of the did_account for further validation
			// when modification is requested
			let id = (&owner, &did_account).using_encoded(blake2_256);
			<OwnerStore<T>>::insert((&owner, &id), did_account);

			Ok(())
		}

		// Update existing attribute on a did
		fn update(
			owner: &T::AccountId,
			did_account: &T::AccountId,
			name: &[u8],
			value: &[u8],
			valid_for: Option<BlockNumberFor<T>>,
		) -> Result<(), DidError> {
			// check if the sender is the owner
			Self::is_owner(owner, did_account)?;

			// validate block number to prevent an overflow
			let validity = match Self::validate_block_number(valid_for) {
				Ok(validity) => validity,
				Err(e) => return Err(e),
			};

			// Get attribute
			let attribute = Self::read(did_account, name);

			match attribute {
				Some(mut attr) => {
					let id = Self::get_hashed_key_for_attr(did_account, name);

					attr.value = value.to_vec();
					attr.validity = validity;

					<AttributeStore<T>>::mutate(id, |a| *a = attr);
					Ok(())
				},
				None => Err(DidError::NotFound),
			}
		}

		// Fetch an attribute from a did
		fn read(
			did_account: &T::AccountId,
			name: &[u8],
		) -> Option<Attribute<BlockNumberFor<T>, <<T as Config>::Time as MomentTime>::Moment>> {
			let id = Self::get_hashed_key_for_attr(did_account, name);

			if <AttributeStore<T>>::contains_key(id) {
				return Some(Self::attribute_of(id));
			}
			None
		}

		// Delete an attribute from a did
		fn delete(
			owner: &T::AccountId,
			did_account: &T::AccountId,
			name: &[u8],
		) -> Result<(), DidError> {
			// check if the sender is the owner
			Self::is_owner(owner, did_account)?;

			let id = Self::get_hashed_key_for_attr(did_account, name);

			if !<AttributeStore<T>>::contains_key(id) {
				return Err(DidError::NotFound);
			}
			<AttributeStore<T>>::remove(id);
			Ok(())
		}

		fn get_hashed_key_for_attr(did_account: &T::AccountId, name: &[u8]) -> [u8; 32] {
			let mut bytes_in_name: Vec<u8> = name.to_vec();
			let mut bytes_to_hash: Vec<u8> = did_account.encode().as_slice().to_vec();
			bytes_to_hash.append(&mut bytes_in_name);
			blake2_256(&bytes_to_hash[..])
		}

		fn validate_block_number(
			valid_for: Option<BlockNumberFor<T>>,
		) -> Result<BlockNumberFor<T>, DidError> {
			let max_block: BlockNumberFor<T> = Bounded::max_value();

			let validity: BlockNumberFor<T> = match valid_for {
				Some(blocks) => {
					let now_block_number: BlockNumberFor<T> =
						<frame_system::Pallet<T>>::block_number();

					// check for addition values overflow
					// new_added_vailidity will be NONE if overflown
					let new_added_vailidity = now_block_number.checked_add(&blocks);

					match new_added_vailidity {
						Some(v) => v,
						None => return Err(DidError::MaxBlockNumberExceeded),
					}
				},
				None => max_block,
			};

			Ok(validity)
		}
	}
}
