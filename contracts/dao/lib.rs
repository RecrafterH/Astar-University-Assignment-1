#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod dao {
    use ink::storage::Mapping;
    use openbrush::contracts::traits::psp22::*;
    use scale::{
        Decode,
        Encode,
    };
    use ink::env::{
        call::{build_call, ExecutionInput, Selector},
        DefaultEnvironment,
    };
    type ProposalId = u64;

    const ONE_MINUTE: u64 = 60;

    #[derive(Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Debug, PartialEq, Eq, scale_info::TypeInfo))]
    pub enum VoteType {
        Against,
        For,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum GovernorError {
        AmountShouldNotBeZero,
        DurationError,
        VotePeriodEnded,
        AlreadyVoted,
        ProposalAlreadyExecuted,
        ProposalNotFound,
        InsufficientBalance,
        QuorumNotReached
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum DaoError {
        AmountShouldNotBeZero,
        DurationError,
        VotePeriodEnded,
        AlreadyVoted,
        ProposalAlreadyExecuted,
        ProposalNotFound,
        InsufficientBalance
    }

    #[derive(Encode, Decode)]
    #[cfg_attr(
        feature = "std",
        derive(
            Debug,
            PartialEq,
            Eq,
            scale_info::TypeInfo,
            ink::storage::traits::StorageLayout
        )
    )]
    pub struct Proposal {
        // to implement
        to: AccountId,
        vote_start: u64,
        vote_end: u64,
        executed: bool,
        amount: Balance, 
    }

    #[derive(Encode, Decode, Default)]
    #[cfg_attr(
        feature = "std",
        derive(
            Debug,
            PartialEq,
            Eq,
            scale_info::TypeInfo,
            ink::storage::traits::StorageLayout
        )
    )]
    pub struct ProposalVote {
        for_votes: u64,
        against_vote: u64,
    }

    #[ink(storage)]
    pub struct Governor {
        governance_token: AccountId,
        quorum: u8,
        next_proposal_id: u64,
        proposals: Mapping<u64 , Proposal>,
        votes: Mapping<(u64, AccountId), bool>,
        proposal_votes : Mapping<Proposal, ProposalVote>,
    }

    impl Governor {
        #[ink(constructor, payable)]
        pub fn new(governance_token: AccountId, quorum: u8) -> Self {
            let next_proposal_id = 0;
            let proposals = Default::default();
            let votes = Default::default();
            let proposal_votes = Default::default();
            Governor {
                governance_token,
                quorum,
                next_proposal_id,
                proposals,
                votes,
                proposal_votes,
            }

        }

        #[ink(message)]
        pub fn propose(
            &mut self,
            to: AccountId,
            amount: Balance,
            duration: u64,
        ) -> Result<(), GovernorError> {

            if amount == 0 {
                return Err(GovernorError::AmountShouldNotBeZero)
            }
            if duration == 0 {
                return Err(GovernorError::DurationError)
            }

            let vote_start = Self::env().block_timestamp();
            let vote_end = vote_start + duration * ONE_MINUTE;
            
            let executed = false;
            let proposal = Proposal {
                to,
                vote_start,
                vote_end,
                executed,
                amount
            };
            self.proposals.insert(self.next_proposal_id , &proposal);
            self.next_proposal_id  += 1;
            Ok(())
        }

        #[ink(message)]
        pub fn vote(
            &mut self,
            proposal_id: ProposalId,
            vote: VoteType,
        ) -> Result<(), GovernorError> {
            let caller = Self::env().caller();
            let proposal_error = self.proposals.get(proposal_id);
            
            let total_supply = 100;/* build_call::<DefaultEnvironment>()
            .call(self.governance_token)
            .gas_limit(5000000000)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("PSP22::total_supply")))
            )
            .returns::<Balance>()
            .try_invoke().unwrap().unwrap(); */
            let caller_balance = 25;/* build_call::<DefaultEnvironment>()
            .call(self.governance_token)
            .gas_limit(5000000000)
            .exec_input(
                ExecutionInput::new(Selector::new(ink::selector_bytes!("PSP22::balance_of")))
                    .push_arg(caller),
            )
            .returns::<Balance>()
            .try_invoke().unwrap().unwrap(); */
            let weight_value = caller_balance * 100/ total_supply;
            if proposal_error == None {
                return Err(GovernorError::ProposalNotFound)
            }
            let proposal = self.proposals.get(proposal_id).unwrap();
            let proposal_executed = proposal.executed;
            if proposal_executed == true {
                return Err(GovernorError::ProposalAlreadyExecuted)
            }
            if proposal.vote_end <= Self::env().block_timestamp() {
                return Err(GovernorError::VotePeriodEnded)
            }
            let voted = self.votes.get((proposal_id, caller));
            if voted == Some(true) {
                return Err(GovernorError::AlreadyVoted)
            }

            self.votes.insert((proposal_id, caller), &true);
            if vote == VoteType::For {
                if self.proposal_votes.get(&proposal) == None {
                    let proposal_vote = ProposalVote {
                        for_votes: weight_value,
                        against_vote: 0,
                    };
                    self.proposal_votes.insert(&proposal, &proposal_vote);
                } else {
                let mut for_amount = self.proposal_votes.get(&proposal).unwrap();
                for_amount.for_votes += weight_value as u64;
                self.proposal_votes.insert(proposal, &for_amount);
                }
                
            } else {
                if self.proposal_votes.get(&proposal) == None {
                    let proposal_vote = ProposalVote {
                        for_votes: 0,
                        against_vote: weight_value,
                    };
                    self.proposal_votes.insert(&proposal, &proposal_vote);
                } else {
                    let mut against_amount = self.proposal_votes.get(&proposal).unwrap();
                    against_amount.against_vote += weight_value as u64;
    
                    self.proposal_votes.insert(proposal, &against_amount);
                }
                

            };
            Ok(())
        }

        #[ink(message)]
        pub fn execute(&mut self, proposal_id: ProposalId) -> Result<(), GovernorError> {
            let caller = Self::env().caller();
            let proposal_error = self.proposals.get(proposal_id);
            
            if proposal_error == None {
                return Err(GovernorError::ProposalNotFound)
            }
            let mut proposal = self.proposals.get(proposal_id).unwrap();
            if proposal.executed == true {
                return Err(GovernorError::ProposalAlreadyExecuted)
            }
            let proposal_votes = self.proposal_votes.get(&proposal);
            if proposal_votes == None {
                return Err(GovernorError::QuorumNotReached)
            }
            let proposal_votes = proposal_votes.unwrap();
            let number_for_votes = proposal_votes.for_votes;
            let number_against_votes = proposal_votes.against_vote;
            let total_votes: u8 = number_for_votes as u8+ number_against_votes as u8;

            if total_votes < self.quorum {
                return Err(GovernorError::QuorumNotReached)
            }

            if number_for_votes > number_against_votes {
                self.env().transfer(caller, proposal.amount);
                proposal.executed = true;
                self.proposals.insert(proposal_id, &proposal);
            }
            Ok(())

        }

        // used for test
        #[ink(message)]
        pub fn now(&self) -> u64 {
            self.env().block_timestamp()
        }

        pub fn get_proposal(&self, proposal_id: ProposalId) -> Option<Proposal> {
            self.proposals.get(proposal_id)
        }

        pub fn next_proposal_id(&self) -> u64 {
            self.next_proposal_id
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn create_contract(initial_balance: Balance) -> Governor {
            let accounts = default_accounts();
            set_sender(accounts.alice);
            set_balance(contract_id(), initial_balance);
            Governor::new(AccountId::from([0x01; 32]), 50)
        }

        fn contract_id() -> AccountId {
            ink::env::test::callee::<ink::env::DefaultEnvironment>()
        }

        fn default_accounts(
        ) -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn set_sender(sender: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(sender);
        }

        fn set_balance(account_id: AccountId, balance: Balance) {
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                account_id, balance,
            )
        }

        #[ink::test]
        fn propose_works() {
            let accounts = default_accounts();
            let mut governor = create_contract(1000);
            assert_eq!(
                governor.propose(accounts.django, 0, 1),
                Err(GovernorError::AmountShouldNotBeZero)
            );
            assert_eq!(
                governor.propose(accounts.django, 100, 0),
                Err(GovernorError::DurationError)
            );
            let result = governor.propose(accounts.django, 100, 1);
            assert_eq!(result, Ok(()));
            let proposal = governor.get_proposal(0).unwrap();
            let now = governor.now();
            assert_eq!(
                proposal,
                Proposal {
                    to: accounts.django,
                    amount: 100,
                    vote_start: 0,
                    vote_end: now + 1 * ONE_MINUTE,
                    executed: false,
                }
            );
            assert_eq!(governor.next_proposal_id(), 1);
        }

        #[ink::test]
        fn quorum_not_reached() {
            let mut governor = create_contract(1000);
            let result = governor.propose(AccountId::from([0x02; 32]), 100, 1);
            assert_eq!(result, Ok(()));
            let execute = governor.execute(0);
            assert_eq!(execute, Err(GovernorError::QuorumNotReached));
        }
    }
}
