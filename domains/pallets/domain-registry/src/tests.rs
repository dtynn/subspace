use crate::{
    self as pallet_domain_registry, DomainConfig, DomainCreators, DomainOperators, Domains, Error,
    NextDomainId,
};
use frame_support::traits::{ConstU16, ConstU32, ConstU64, GenesisBuild};
use frame_support::{assert_noop, assert_ok, parameter_types};
use pallet_balances::AccountData;
use sp_core::crypto::Pair;
use sp_core::{H256, U256};
use sp_domains::{ExecutorPair, StakeWeight};
use sp_runtime::testing::Header;
use sp_runtime::traits::{BlakeTwo256, IdentityLookup};
use sp_runtime::Percent;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub struct Test
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        ExecutorRegistry: pallet_executor_registry,
        DomainRegistry: pallet_domain_registry,
    }
);

type AccountId = u64;
type BlockNumber = u64;
type Balance = u128;
type Hash = H256;

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = ConstU64<2>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub static ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinExecutorStake: Balance = 10;
    pub const MaxExecutorStake: Balance = 1000;
    pub const MinExecutors: u32 = 1;
    pub const MaxExecutors: u32 = 10;
    pub const EpochDuration: BlockNumber = 3;
    pub const MaxWithdrawals: u32 = 1;
    pub const WithdrawalDuration: BlockNumber = 10;
}

impl pallet_executor_registry::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type StakeWeight = StakeWeight;
    type MinExecutorStake = MinExecutorStake;
    type MaxExecutorStake = MaxExecutorStake;
    type MinExecutors = MinExecutors;
    type MaxExecutors = MaxExecutors;
    type EpochDuration = EpochDuration;
    type MaxWithdrawals = MaxWithdrawals;
    type WithdrawalDuration = WithdrawalDuration;
}

impl crate::ExecutorRegistry<AccountId, Balance> for ExecutorRegistry {
    fn executor_stake(who: &AccountId) -> Option<Balance> {
        ExecutorRegistry::executor_stake(who)
    }
}

parameter_types! {
    pub const MinDomainDeposit: Balance = 10;
    pub const MaxDomainDeposit: Balance = 1000;
    pub const MinDomainOperatorStake: u32 = 10;
}

impl pallet_domain_registry::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type ExecutorRegistry = ExecutorRegistry;
    type MinDomainDeposit = MinDomainDeposit;
    type MaxDomainDeposit = MaxDomainDeposit;
    type MinDomainOperatorStake = MinDomainOperatorStake;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, 1000), (2, 2000), (3, 3000), (4, 4000)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    pallet_executor_registry::GenesisConfig::<Test> {
        executors: vec![
            (
                1,
                100,
                1 + 10000,
                ExecutorPair::from_seed(&U256::from(1u32).into()).public(),
            ),
            (
                2,
                200,
                2 + 10000,
                ExecutorPair::from_seed(&U256::from(2u32).into()).public(),
            ),
        ],
        slot_probability: (1u64, 1u64),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    pallet_domain_registry::GenesisConfig::<Test> {
        domains: vec![(
            1,
            100,
            DomainConfig {
                wasm_runtime_hash: Hash::repeat_byte(1),
                bundle_frequency: 100,
                max_bundle_size: 1024 * 1024,
                max_bundle_weight: 100_000_000_000,
                min_operator_stake: 20,
            },
            1,
            Percent::from_percent(80),
        )],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}

#[test]
fn create_domain_should_work() {
    new_test_ext().execute_with(|| {
        let genesis_domain_config = DomainConfig {
            wasm_runtime_hash: Hash::repeat_byte(1),
            bundle_frequency: 100,
            max_bundle_size: 1024 * 1024,
            max_bundle_weight: 100_000_000_000,
            min_operator_stake: 20,
        };
        assert_eq!(Domains::<Test>::get(0).unwrap(), genesis_domain_config);
        assert_eq!(NextDomainId::<Test>::get(), 1);
        assert_eq!(DomainCreators::<Test>::get(0, 1), Some(100));

        let domain_config = DomainConfig {
            wasm_runtime_hash: Hash::random(),
            bundle_frequency: 100,
            max_bundle_size: 1024 * 1024,
            max_bundle_weight: 100_000_000_000,
            min_operator_stake: 20,
        };

        assert_noop!(
            DomainRegistry::create_domain(Origin::signed(1), 1, domain_config.clone()),
            Error::<Test>::DepositTooSmall
        );
        assert_noop!(
            DomainRegistry::create_domain(Origin::signed(1), 10_000, domain_config.clone()),
            Error::<Test>::DepositTooLarge
        );
        assert_noop!(
            DomainRegistry::create_domain(Origin::signed(8), 100, domain_config.clone()),
            Error::<Test>::InsufficientBalance
        );
        assert_noop!(
            DomainRegistry::create_domain(
                Origin::signed(1),
                100,
                DomainConfig {
                    min_operator_stake: 1,
                    ..domain_config
                }
            ),
            Error::<Test>::OperatorStakeThresholdTooLow
        );

        let (creator, deposit) = (2, 200);
        let next_domain_id = NextDomainId::<Test>::get();
        assert_ok!(DomainRegistry::create_domain(
            Origin::signed(creator),
            deposit,
            domain_config.clone(),
        ));
        assert_eq!(
            frame_system::Account::<Test>::get(&creator).data,
            AccountData {
                free: 2000,
                reserved: 0,
                misc_frozen: deposit,
                fee_frozen: deposit
            }
        );

        assert_eq!(Domains::<Test>::get(next_domain_id).unwrap(), domain_config);
        assert_eq!(NextDomainId::<Test>::get(), next_domain_id + 1);
        assert_eq!(
            DomainCreators::<Test>::get(next_domain_id, creator),
            Some(deposit)
        );
    });
}

#[test]
fn register_domain_operator_and_update_domain_stake_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            DomainOperators::<Test>::get(1, 0).unwrap(),
            Percent::from_percent(80)
        );

        assert_noop!(
            DomainRegistry::update_domain_stake(Origin::signed(1), 0, Percent::from_percent(10),),
            Error::<Test>::OperatorStakeTooSmall
        );

        assert_ok!(DomainRegistry::update_domain_stake(
            Origin::signed(1),
            0,
            Percent::from_percent(20),
        ));

        assert_eq!(
            DomainOperators::<Test>::get(1, 0).unwrap(),
            Percent::from_percent(20)
        );

        assert_ok!(DomainRegistry::create_domain(
            Origin::signed(2),
            200,
            DomainConfig {
                wasm_runtime_hash: Hash::random(),
                bundle_frequency: 100,
                max_bundle_size: 1024 * 1024,
                max_bundle_weight: 100_000_000_000,
                min_operator_stake: 20,
            }
        ));

        // only 80% is available.
        assert_noop!(
            DomainRegistry::register_domain_operator(
                Origin::signed(1),
                1,
                Percent::from_percent(90),
            ),
            Error::<Test>::StakeAllocationTooLarge
        );

        assert_ok!(DomainRegistry::register_domain_operator(
            Origin::signed(1),
            1,
            Percent::from_percent(80),
        ));

        assert_ok!(DomainRegistry::register_domain_operator(
            Origin::signed(2),
            0,
            Percent::from_percent(50),
        ));

        assert_eq!(
            DomainOperators::<Test>::get(2, 0).unwrap(),
            Percent::from_percent(50)
        );

        assert_noop!(
            DomainRegistry::register_domain_operator(
                Origin::signed(3),
                1,
                Percent::from_percent(30),
            ),
            Error::<Test>::NotExecutor
        );
    });
}

#[test]
fn deregister_domain_operator_should_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            DomainRegistry::deregister_domain_operator(Origin::signed(3), 0,),
            Error::<Test>::NotOperator
        );

        assert_ok!(DomainRegistry::register_domain_operator(
            Origin::signed(2),
            0,
            Percent::from_percent(50),
        ));

        assert_ok!(DomainRegistry::deregister_domain_operator(
            Origin::signed(1),
            0,
        ));

        assert!(DomainOperators::<Test>::get(1, 0).is_none());
    });
}