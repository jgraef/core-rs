pub mod tree;
pub mod basic_account;
pub mod htlc_contract;
pub mod vesting_contract;
pub mod accounts;

use beserial::{Deserialize, Serialize, SerializingError, WriteBytesExt, ReadBytesExt};
use crate::consensus::base::transaction::{Transaction, TransactionError};
use crate::consensus::base::primitive::{Address, Coin};
use crate::consensus::base::primitive::hash::{Hash, HashOutput, Hasher, SerializeContent};
use std::cmp::Ordering;
use std::io;
use std::fmt;

pub use self::basic_account::BasicAccount;
pub use self::htlc_contract::HashedTimeLockedContract;
pub use self::vesting_contract::VestingContract;
pub use self::accounts::Accounts;

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum AccountType {
    Basic = 0,
    Vesting = 1,
    HTLC = 2,
}

macro_rules! invoke_account_type {
    ($on: expr, $name: ident, $( $arg: ident ),*) => {
        match $on {
            AccountType::Basic => BasicAccount::$name($( $arg ),*),
            AccountType::Vesting => VestingContract::$name($( $arg ),*),
            AccountType::HTLC => HashedTimeLockedContract::$name($( $arg ),*),
        }
    }
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub enum Account {
    Basic(BasicAccount),
    Vesting(VestingContract),
    HTLC(HashedTimeLockedContract),
}

macro_rules! invoke_account_instance {
    ($on: expr, $name: ident, $( $arg: ident ),*) => {
        match $on {
            Account::Basic(ref account) => Ok(Account::Basic(account.$name($( $arg ),*)?)),
            Account::Vesting(ref account) => Ok(Account::Vesting(account.$name($( $arg ),*)?)),
            Account::HTLC(ref account) => Ok(Account::HTLC(account.$name($( $arg ),*)?)),
        }
    }
}

impl Account {
    const INITIAL: Account = Account::Basic(BasicAccount { balance: Coin::ZERO });

    pub fn new_basic(balance: Coin) -> Account {
        return Account::Basic(BasicAccount { balance });
    }

    pub fn new_contract(account_type: AccountType, balance: Coin, transaction: &Transaction, block_height: u32) -> Result<Self, AccountError> {
        return match account_type {
            AccountType::Basic => Err(AccountError::InvalidForRecipient),
            AccountType::Vesting => Ok(Account::Vesting(VestingContract::create(balance, transaction, block_height)?)),
            AccountType::HTLC => Ok(Account::HTLC(HashedTimeLockedContract::create(balance, transaction, block_height)?))
        };
    }

    pub fn verify_incoming_transaction(transaction: &Transaction) -> Result<(), TransactionError> {
        invoke_account_type!(transaction.recipient_type, verify_incoming_transaction, transaction)
    }

    pub fn verify_outgoing_transaction(transaction: &Transaction) -> Result<(), TransactionError> {
        invoke_account_type!(transaction.sender_type, verify_outgoing_transaction, transaction)
    }

    pub fn with_incoming_transaction(&self, transaction: &Transaction, block_height: u32) -> Result<Self, AccountError> {
        invoke_account_instance!(*self, with_incoming_transaction, transaction, block_height)
    }

    pub fn without_incoming_transaction(&self, transaction: &Transaction, block_height: u32) -> Result<Self, AccountError> {
        invoke_account_instance!(*self, without_incoming_transaction, transaction, block_height)
    }

    pub fn with_outgoing_transaction(&self, transaction: &Transaction, block_height: u32) -> Result<Self, AccountError> {
        // Check account balance.
        // This assumes that transaction.value + transaction.fee does not overflow.
        let balance = self.balance();
        if balance < transaction.value + transaction.fee {
            return Err(AccountError::InsufficientFunds);
        }

        invoke_account_instance!(*self, with_outgoing_transaction, transaction, block_height)
    }

    pub fn without_outgoing_transaction(&self, transaction: &Transaction, block_height: u32) -> Result<Self, AccountError> {
        invoke_account_instance!(*self, without_outgoing_transaction, transaction, block_height)
    }

    pub fn account_type(&self) -> AccountType {
        return match *self {
            Account::Basic(_) => AccountType::Basic,
            Account::Vesting(_) => AccountType::Vesting,
            Account::HTLC(_) => AccountType::HTLC
        };
    }

    pub fn balance(&self) -> Coin {
        return match *self {
            Account::Basic(ref account) => account.balance,
            Account::Vesting(ref account) => account.balance,
            Account::HTLC(ref account) => account.balance
        };
    }

    pub fn is_initial(&self) -> bool {
        return match *self {
            Account::Basic(ref account) => account.balance == Coin::ZERO,
            _ => false
        };
    }

    pub fn is_to_be_pruned(&self) -> bool {
        return match *self {
            Account::Basic(_) => false,
            _ => self.balance() == Coin::ZERO,
        };
    }

    pub fn balance_add(balance: Coin, value: Coin) -> Result<Coin, AccountError> {
        return match balance.checked_add(value) {
            Some(result) => Ok(result),
            None => Err(AccountError::InsufficientFunds)
        };
    }

    pub fn balance_sub(balance: Coin, value: Coin) -> Result<Coin, AccountError> {
        return match balance.checked_sub(value) {
            Some(result) => Ok(result),
            None => Err(AccountError::InsufficientFunds)
        };
    }
}

impl Serialize for Account {
    fn serialize<W: WriteBytesExt>(&self, writer: &mut W) -> Result<usize, SerializingError> {
        let mut size: usize = 0;
        size += Serialize::serialize(&self.account_type(), writer)?;

        match *self {
            Account::Basic(ref account) => {
                size += Serialize::serialize(&account, writer)?;
            }
            Account::Vesting(ref account) => {
                size += Serialize::serialize(&account, writer)?;
            }
            Account::HTLC(ref account) => {
                size += Serialize::serialize(&account, writer)?;
            }
        }

        return Ok(size);
    }

    fn serialized_size(&self) -> usize {
        let mut size = /*type*/ 1;

        match *self {
            Account::Basic(ref account) => {
                size += Serialize::serialized_size(&account);
            }
            Account::Vesting(ref account) => {
                size += Serialize::serialized_size(&account);
            }
            Account::HTLC(ref account) => {
                size += Serialize::serialized_size(&account);
            }
        }

        return size;
    }
}

impl Deserialize for Account {
    fn deserialize<R: ReadBytesExt>(reader: &mut R) -> Result<Self, SerializingError> {
        let account_type: AccountType = Deserialize::deserialize(reader)?;

        match account_type {
            AccountType::Basic => {
                let account: BasicAccount = Deserialize::deserialize(reader)?;
                return Ok(Account::Basic(account));
            }
            AccountType::Vesting => {
                let account: VestingContract = Deserialize::deserialize(reader)?;
                return Ok(Account::Vesting(account));
            }
            AccountType::HTLC => {
                let account: HashedTimeLockedContract = Deserialize::deserialize(reader)?;
                return Ok(Account::HTLC(account));
            }
        }
    }
}

#[derive(Clone, Eq, Debug, Serialize, Deserialize)]
pub struct PrunedAccount {
    pub address: Address,
    pub account: Account,
}

impl SerializeContent for PrunedAccount {
    fn serialize_content<W: io::Write>(&self, writer: &mut W) -> io::Result<usize> { Ok(self.serialize(writer)?) }
}

impl Hash for PrunedAccount {
    fn hash<H: HashOutput>(&self) -> H  {
        let h = H::Builder::default();
        self.serialize_content(&mut vec![]).unwrap();
        return h.finish();
    }
}

impl Ord for PrunedAccount {
    fn cmp(&self, other: &PrunedAccount) -> Ordering {
        self.address.cmp(&other.address)
    }
}

impl PartialOrd for PrunedAccount {
    fn partial_cmp(&self, other: &PrunedAccount) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PrunedAccount {
    fn eq(&self, other: &PrunedAccount) -> bool {
        self.address == other.address
    }
}


#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub enum AccountError {
    InsufficientFunds,
    TypeMismatch,
    InvalidSignature,
    InvalidForSender,
    InvalidForRecipient,
    InvalidPruning,
    InvalidSerialization(SerializingError),
    InvalidTransaction(TransactionError),
    AccountsHashMismatch, // XXX This doesn't really belong here
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: Don't use debug formatter
        return write!(f, "{:?}", self);
    }
}

impl From<SerializingError> for AccountError {
    fn from(e: SerializingError) -> Self {
        AccountError::InvalidSerialization(e)
    }
}

impl From<TransactionError> for AccountError {
    fn from(e: TransactionError) -> Self {
        AccountError::InvalidTransaction(e)
    }
}
