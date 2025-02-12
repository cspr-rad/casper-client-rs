use std::fmt::{self, Display, Formatter};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use casper_hashing::Digest;
use casper_types::{
    bytesrepr::{self, ToBytes},
    crypto, PublicKey, SecretKey, Signature, URef, U512,
};

use crate::{
    types::{ExecutableDeployItem, TimeDiff, Timestamp},
    Error, TransferTarget,
};

/// The maximum permissible size in bytes of a Deploy when serialized via `ToBytes`.
///
/// Note: this should be kept in sync with the value of `[deploys.max_deploy_size]` in the
/// production chainspec.
pub const MAX_SERIALIZED_SIZE_OF_DEPLOY: u32 = 3_072 * 1_024;

/// A cryptographic hash uniquely identifying a [`Deploy`].
#[derive(
    Copy, Clone, Default, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug,
)]
#[serde(deny_unknown_fields)]
pub struct DeployHash(Digest);

impl DeployHash {
    /// Returns a new `DeployHash`.
    pub fn new(digest: Digest) -> Self {
        DeployHash(digest)
    }

    /// Returns a copy of the wrapped `Digest`.
    pub fn inner(&self) -> Digest {
        self.0
    }
}

impl Display for DeployHash {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl AsRef<[u8]> for DeployHash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl ToBytes for DeployHash {
    fn write_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), bytesrepr::Error> {
        self.0.write_bytes(buffer)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        self.0.to_bytes()
    }

    fn serialized_length(&self) -> usize {
        self.0.serialized_length()
    }
}

/// The header portion of a [`Deploy`].
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct DeployHeader {
    account: PublicKey,
    timestamp: Timestamp,
    ttl: TimeDiff,
    gas_price: u64,
    body_hash: Digest,
    dependencies: Vec<DeployHash>,
    chain_name: String,
}

impl DeployHeader {
    /// Returns the account within which the deploy will be run.
    pub fn account(&self) -> &PublicKey {
        &self.account
    }

    /// Returns the deploy creation timestamp.
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    /// Returns the duration for which the deploy will stay valid.
    pub fn ttl(&self) -> TimeDiff {
        self.ttl
    }

    /// Returns the price per gas unit for this deploy.
    pub fn gas_price(&self) -> u64 {
        self.gas_price
    }

    /// Returns the hash of the body of this deploy.
    pub fn body_hash(&self) -> Digest {
        self.body_hash
    }

    /// Returns the list of other deploys that have to be run before this one.
    pub fn dependencies(&self) -> impl Iterator<Item = &DeployHash> {
        self.dependencies.iter()
    }

    /// Returns the chain name of the network the deploy is supposed to be run on.
    pub fn chain_name(&self) -> &str {
        &self.chain_name
    }
}

impl Display for DeployHeader {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(
            formatter,
            "deploy header {{ account {}, timestamp {}, ttl {}, body hash {}, chain name {} }}",
            self.account, self.timestamp, self.ttl, self.body_hash, self.chain_name,
        )
    }
}

impl ToBytes for DeployHeader {
    fn write_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), bytesrepr::Error> {
        self.account.write_bytes(buffer)?;
        self.timestamp.write_bytes(buffer)?;
        self.ttl.write_bytes(buffer)?;
        self.gas_price.write_bytes(buffer)?;
        self.body_hash.write_bytes(buffer)?;
        self.dependencies.write_bytes(buffer)?;
        self.chain_name.write_bytes(buffer)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut buffer = vec![];
        self.write_bytes(&mut buffer)?;
        Ok(buffer)
    }

    fn serialized_length(&self) -> usize {
        self.account.serialized_length()
            + self.timestamp.serialized_length()
            + self.ttl.serialized_length()
            + self.gas_price.serialized_length()
            + self.body_hash.serialized_length()
            + self.dependencies.serialized_length()
            + self.chain_name.serialized_length()
    }
}

/// The signature of a deploy and the public key of the signer.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    signer: PublicKey,
    signature: Signature,
}

impl Approval {
    /// Returns the public key.
    pub fn signer(&self) -> &PublicKey {
        &self.signer
    }

    /// Returns the signature.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }
}

impl ToBytes for Approval {
    fn write_bytes(&self, buffer: &mut Vec<u8>) -> Result<(), bytesrepr::Error> {
        self.signer.write_bytes(buffer)?;
        self.signature.write_bytes(buffer)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut buffer = vec![];
        self.write_bytes(&mut buffer)?;
        Ok(buffer)
    }

    fn serialized_length(&self) -> usize {
        self.signer.serialized_length() + self.signature.serialized_length()
    }
}

/// A signed item sent to the network used to request execution of Wasm.
///
/// Note that constructing a `Deploy` is done via the [`DeployBuilder`].
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Deploy {
    /// The unique identifier for the deploy.
    pub hash: DeployHash,

    /// The header information for the deploy.
    pub header: DeployHeader,

    /// The payment logic for the deploy.
    pub payment: ExecutableDeployItem,

    /// The session logic for the deploy.
    pub session: ExecutableDeployItem,

    /// List of approvals for the deploy.
    pub approvals: Vec<Approval>,
}

/// Used when constructing a `Deploy`.
#[derive(Debug)]
pub enum AccountAndSecretKey<'a> {
    /// Provides both the account and the secret key (not necessarily for the same account) used to
    /// sign the `Deploy`.
    Both {
        /// The public key of the account.
        account: PublicKey,
        /// The secret key used to sign the `Deploy`.
        secret_key: &'a SecretKey,
    },
    /// The public key of the account.  The `Deploy` will be created unsigned as no secret key is
    /// provided.
    Account(PublicKey),
    /// The account will be derived from the provided secret key, and the `Deploy` will be signed by
    /// the same secret key.
    SecretKey(&'a SecretKey),
}

impl<'a> AccountAndSecretKey<'a> {
    fn account(&self) -> PublicKey {
        match self {
            AccountAndSecretKey::Both { account, .. } | AccountAndSecretKey::Account(account) => {
                account.clone()
            }
            AccountAndSecretKey::SecretKey(secret_key) => PublicKey::from(*secret_key),
        }
    }

    fn secret_key(&self) -> Option<&SecretKey> {
        match self {
            AccountAndSecretKey::Both { secret_key, .. }
            | AccountAndSecretKey::SecretKey(secret_key) => Some(secret_key),
            AccountAndSecretKey::Account(_) => None,
        }
    }
}

impl Deploy {
    /// The default time-to-live for `Deploy`s, i.e. 30 minutes.
    pub const DEFAULT_TTL: TimeDiff = TimeDiff::from_millis(30 * 60 * 1_000);
    /// The default gas price for `Deploy`s, i.e. `1`.
    pub const DEFAULT_GAS_PRICE: u64 = 1;

    /// Constructs a new signed `Deploy`.
    #[allow(clippy::too_many_arguments)]
    fn new(
        timestamp: Timestamp,
        ttl: TimeDiff,
        gas_price: u64,
        dependencies: Vec<DeployHash>,
        chain_name: String,
        payment: ExecutableDeployItem,
        session: ExecutableDeployItem,
        account_and_secret_key: AccountAndSecretKey,
    ) -> Deploy {
        let serialized_body = serialize_body(&payment, &session);
        let body_hash = Digest::hash(serialized_body);

        let account = account_and_secret_key.account();

        // Remove duplicates.
        let dependencies = dependencies.into_iter().unique().collect();
        let header = DeployHeader {
            account,
            timestamp,
            ttl,
            gas_price,
            body_hash,
            dependencies,
            chain_name,
        };
        let serialized_header = header
            .to_bytes()
            .unwrap_or_else(|error| panic!("should serialize deploy header: {}", error));
        let hash = DeployHash(Digest::hash(serialized_header));

        let mut deploy = Deploy {
            hash,
            header,
            payment,
            session,
            approvals: vec![],
        };

        if let Some(secret_key) = account_and_secret_key.secret_key() {
            deploy.sign(secret_key);
        }
        deploy
    }

    /// Adds a signature of this deploy's hash to its approvals.
    pub fn sign(&mut self, secret_key: &SecretKey) {
        let signer = PublicKey::from(secret_key);
        let signature = crypto::sign(self.hash.0, secret_key, &signer);
        let approval = Approval { signer, signature };
        self.approvals.push(approval);
    }

    /// Returns `Ok` if the serialized size of the deploy is not greater than `max_deploy_size`.
    pub fn is_valid_size(&self, max_deploy_size: u32) -> Result<(), Error> {
        let deploy_size = self.header.serialized_length()
            + self.hash.serialized_length()
            + self.payment.serialized_length()
            + self.session.serialized_length()
            + self.approvals.serialized_length();
        if deploy_size > max_deploy_size as usize {
            return Err(Error::DeploySizeTooLarge {
                max_deploy_size,
                actual_deploy_size: deploy_size,
            });
        }
        Ok(())
    }

    /// Returns the hash uniquely identifying this deploy.
    pub fn id(&self) -> &DeployHash {
        &self.hash
    }

    /// Returns the header portion of the deploy.
    pub fn header(&self) -> &DeployHeader {
        &self.header
    }

    /// Returns the payment code of the deploy.
    pub fn payment(&self) -> &ExecutableDeployItem {
        &self.payment
    }

    /// Returns the session code of the deploy.
    pub fn session(&self) -> &ExecutableDeployItem {
        &self.session
    }

    /// Returns the approvals; the public keys and signatures of the signatories of the deploy.
    pub fn approvals(&self) -> &[Approval] {
        &self.approvals
    }
}

impl Display for Deploy {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "deploy {{ {}, account {}, timestamp {}, ttl {}, body hash {}, chain name {} }}",
            self.hash,
            self.header.account,
            self.header.timestamp,
            self.header.ttl,
            self.header.body_hash,
            self.header.chain_name
        )
    }
}

fn serialize_body(payment: &ExecutableDeployItem, session: &ExecutableDeployItem) -> Vec<u8> {
    let mut buffer = payment
        .to_bytes()
        .unwrap_or_else(|error| panic!("should serialize payment code: {}", error));
    buffer.extend(
        session
            .to_bytes()
            .unwrap_or_else(|error| panic!("should serialize session code: {}", error)),
    );
    buffer
}

/// A builder for constructing a [`Deploy`].
pub struct DeployBuilder<'a> {
    account: Option<PublicKey>,
    secret_key: Option<&'a SecretKey>,
    timestamp: Option<Timestamp>,
    ttl: TimeDiff,
    gas_price: u64,
    dependencies: Vec<DeployHash>,
    chain_name: String,
    payment: Option<ExecutableDeployItem>,
    session: ExecutableDeployItem,
}

impl<'a> DeployBuilder<'a> {
    /// Returns a new `DeployBuilder`.
    ///
    /// # Note
    ///
    /// Before calling [`build`](Self::build), you must ensure
    ///   * that an account is provided by either calling [`with_account`](Self::with_account) or
    ///     [`with_secret_key`](Self::with_secret_key)
    ///   * that payment code is provided by either calling
    ///     [`with_standard_payment`](Self::with_standard_payment) or
    ///     [`with_payment`](Self::with_payment)
    ///   * if feature `std-fs-io` is not enabled (it is enabled by default) that a timestamp is
    ///     provided by calling [`with_timestamp`](Self::with_timestamp)
    pub fn new<C: Into<String>>(chain_name: C, session: ExecutableDeployItem) -> Self {
        // Timestamp::now() not available with feature sdk, use default() instead
        #[cfg(feature = "std-fs-io")]
        let timestamp = Some(Timestamp::now());
        #[cfg(not(feature = "std-fs-io"))]
        let timestamp = None;

        DeployBuilder {
            account: None,
            secret_key: None,
            timestamp,
            ttl: Deploy::DEFAULT_TTL,
            gas_price: Deploy::DEFAULT_GAS_PRICE,
            dependencies: vec![],
            chain_name: chain_name.into(),
            payment: None,
            session,
        }
    }

    /// Returns a new `DeployBuilder` with session code suitable for a transfer.
    ///
    /// If `maybe_source` is None, the account's main purse is used as the source of the transfer.
    ///
    /// # Note
    ///
    /// Before calling [`build`](Self::build), you must ensure
    ///   * that an account is provided by either calling [`with_account`](Self::with_account) or
    ///     [`with_secret_key`](Self::with_secret_key)
    ///   * that payment code is provided by either calling
    ///     [`with_standard_payment`](Self::with_standard_payment) or
    ///     [`with_payment`](Self::with_payment)
    ///   * if feature `std-fs-io` is not enabled (it is enabled by default) that a timestamp is
    ///     provided by calling [`with_timestamp`](Self::with_timestamp)
    pub fn new_transfer<C: Into<String>, A: Into<U512>>(
        chain_name: C,
        amount: A,
        maybe_source: Option<URef>,
        target: TransferTarget,
        maybe_transfer_id: Option<u64>,
    ) -> Self {
        let session =
            ExecutableDeployItem::new_transfer(amount, maybe_source, target, maybe_transfer_id);
        DeployBuilder::new(chain_name, session)
    }

    /// Sets the `account` in the `Deploy`.
    ///
    /// If not provided, the public key derived from the secret key used in the `DeployBuilder` will
    /// be used as the `account` in the `Deploy`.
    pub fn with_account(mut self, account: PublicKey) -> Self {
        self.account = Some(account);
        self
    }

    /// Sets the secret key used to sign the `Deploy` on calling [`build`](Self::build).
    ///
    /// If not provided, the `Deploy` can still be built, but will be unsigned and will be invalid
    /// until subsequently signed.
    pub fn with_secret_key(mut self, secret_key: &'a SecretKey) -> Self {
        self.secret_key = Some(secret_key);
        self
    }

    /// Sets the `payment` in the `Deploy` to a standard payment with the given amount.
    pub fn with_standard_payment<A: Into<U512>>(mut self, amount: A) -> Self {
        self.payment = Some(ExecutableDeployItem::new_standard_payment(amount));
        self
    }

    /// Sets the `payment` in the `Deploy`.
    pub fn with_payment(mut self, payment: ExecutableDeployItem) -> Self {
        self.payment = Some(payment);
        self
    }

    /// Sets the `timestamp` in the `Deploy`.
    ///
    /// If not provided, the timestamp will be set to the time when the `DeployBuilder` was
    /// constructed if feature `std-fs-io` is enabled (it is enabled by default).  If `std-fs-io`
    /// is not enabled, [`build`](Self::build) will return an error if `with_timestamp` has not
    /// previously been called.
    pub fn with_timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the `ttl` (time-to-live) in the `Deploy`.
    ///
    /// If not provided, the ttl will be set to [`Deploy::DEFAULT_TTL`].
    pub fn with_ttl(mut self, ttl: TimeDiff) -> Self {
        self.ttl = ttl;
        self
    }

    /// Returns the new `Deploy`.
    ///
    /// Returns an error if
    ///   * neither [`with_standard_payment`](Self::with_standard_payment) nor
    ///     [`with_payment`](Self::with_payment) were previously called
    ///   * if feature `std-fs-io` was not enabled (it is enabled by default) and
    ///     [`with_timestamp`](Self::with_timestamp) was not previously called
    pub fn build(self) -> Result<Deploy, Error> {
        let account_and_secret_key = match (self.account, self.secret_key) {
            (Some(account), Some(secret_key)) => AccountAndSecretKey::Both {
                account,
                secret_key,
            },
            (Some(account), None) => AccountAndSecretKey::Account(account),
            (None, Some(secret_key)) => AccountAndSecretKey::SecretKey(secret_key),
            (None, None) => return Err(Error::DeployMissingSessionAccount),
        };

        let timestamp = self.timestamp.ok_or(Error::DeployMissingTimestamp)?;
        let payment = self.payment.ok_or(Error::DeployMissingPaymentCode)?;
        let deploy = Deploy::new(
            timestamp,
            self.ttl,
            self.gas_price,
            self.dependencies,
            self.chain_name,
            payment,
            self.session,
            account_and_secret_key,
        );
        Ok(deploy)
    }
}
