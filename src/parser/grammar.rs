// auto-generated: "lalrpop 0.22.2"
// sha3: 8e8b242c3eaaabe55bb101585de8ee66dc30439b7732a5470a07a8596393bf69
#[allow(unused_extern_crates)]
extern crate lalrpop_util as __lalrpop_util;
#[allow(unused_imports)]
use self::__lalrpop_util::state_machine as __state_machine;
#[allow(unused_extern_crates)]
extern crate alloc;

#[rustfmt::skip]
#[allow(explicit_outlives_requirements, non_snake_case, non_camel_case_types, unused_mut, unused_variables, unused_imports, unused_parens, clippy::needless_lifetimes, clippy::type_complexity, clippy::needless_return, clippy::too_many_arguments, clippy::match_single_binding)]
mod __parse__Term {

    #[allow(unused_extern_crates)]
    extern crate lalrpop_util as __lalrpop_util;
    #[allow(unused_imports)]
    use self::__lalrpop_util::state_machine as __state_machine;
    #[allow(unused_extern_crates)]
    extern crate alloc;
    use super::__ToTriple;
    #[allow(dead_code)]
    pub(crate) enum __Symbol<>
     {
        Variant0(crate::lexer::Token<'input>),
        Variant1(i32),
    }
    const __ACTION: &[i8] = &[
        // State 0
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0,
        // State 1
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 2
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0,
        // State 3
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0,
        // State 4
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 5
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // State 6
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    fn __action(state: i8, integer: usize) -> i8 {
        __ACTION[(state as usize) * 57 + integer]
    }
    const __EOF_ACTION: &[i8] = &[
        // State 0
        0,
        // State 1
        -4,
        // State 2
        0,
        // State 3
        0,
        // State 4
        -1,
        // State 5
        -2,
        // State 6
        -3,
    ];
    fn __goto(state: i8, nt: usize) -> i8 {
        match nt {
            0 => 1,
            _ => 0,
        }
    }
    #[allow(clippy::needless_raw_string_hashes)]
    const __TERMINAL: &[&str] = &[
        r###""module""###,
        r###""function""###,
        r###""if""###,
        r###""else""###,
        r###""let""###,
        r###""assert""###,
        r###""echo""###,
        r###""for""###,
        r###""each""###,
        r###""true""###,
        r###""false""###,
        r###""undef""###,
        r###""<=""###,
        r###"">=""###,
        r###""==""###,
        r###""!=""###,
        r###""&&""###,
        r###""||""###,
        r###""<<""###,
        r###"">>""###,
        r###""include""###,
        r###""use""###,
        r###""<""###,
        r###"">""###,
        r###""+""###,
        r###""-""###,
        r###""*""###,
        r###""/""###,
        r###""%""###,
        r###""^""###,
        r###""=""###,
        r###""!""###,
        r###""~""###,
        r###""&""###,
        r###""|""###,
        r###""?""###,
        r###"":""###,
        r###"".""###,
        r###""#""###,
        r###"";""###,
        r###""{""###,
        r###""}""###,
        r###""(""###,
        r###"")""###,
        r###""[""###,
        r###""]""###,
        r###"",""###,
        r###""\u{3}""###,
        r###""\u{a0}""###,
        r###""\u{feff}""###,
        r###""\n""###,
        r###""number""###,
        r###""float""###,
        r###""hex""###,
        r###""ident""###,
        r###""string""###,
        r###""path""###,
    ];
    fn __expected_tokens(__state: i8) -> alloc::vec::Vec<alloc::string::String> {
        __TERMINAL.iter().enumerate().filter_map(|(index, terminal)| {
            let next_state = __action(__state, index);
            if next_state == 0 {
                None
            } else {
                Some(alloc::string::ToString::to_string(terminal))
            }
        }).collect()
    }
    fn __expected_tokens_from_states<
    >(
        __states: &[i8],
        _: core::marker::PhantomData<()>,
    ) -> alloc::vec::Vec<alloc::string::String>
    {
        __TERMINAL.iter().enumerate().filter_map(|(index, terminal)| {
            if __accepts(None, __states, Some(index), core::marker::PhantomData::<()>) {
                Some(alloc::string::ToString::to_string(terminal))
            } else {
                None
            }
        }).collect()
    }
    struct __StateMachine<>
    where 
    {
        __phantom: core::marker::PhantomData<()>,
    }
    impl<> __state_machine::ParserDefinition for __StateMachine<>
    where 
    {
        type Location = text_size::TextSize;
        type Error = lalrpop_util::ParseError<usize, crate::lexer::Token<'static>, &'static str>;
        type Token = crate::lexer::Token<'input>;
        type TokenIndex = usize;
        type Symbol = __Symbol<>;
        type Success = i32;
        type StateIndex = i8;
        type Action = i8;
        type ReduceIndex = i8;
        type NonterminalIndex = usize;

        #[inline]
        fn start_location(&self) -> Self::Location {
              Default::default()
        }

        #[inline]
        fn start_state(&self) -> Self::StateIndex {
              0
        }

        #[inline]
        fn token_to_index(&self, token: &Self::Token) -> Option<usize> {
            __token_to_integer(token, core::marker::PhantomData::<()>)
        }

        #[inline]
        fn action(&self, state: i8, integer: usize) -> i8 {
            __action(state, integer)
        }

        #[inline]
        fn error_action(&self, state: i8) -> i8 {
            __action(state, 57 - 1)
        }

        #[inline]
        fn eof_action(&self, state: i8) -> i8 {
            __EOF_ACTION[state as usize]
        }

        #[inline]
        fn goto(&self, state: i8, nt: usize) -> i8 {
            __goto(state, nt)
        }

        fn token_to_symbol(&self, token_index: usize, token: Self::Token) -> Self::Symbol {
            __token_to_symbol(token_index, token, core::marker::PhantomData::<()>)
        }

        fn expected_tokens(&self, state: i8) -> alloc::vec::Vec<alloc::string::String> {
            __expected_tokens(state)
        }

        fn expected_tokens_from_states(&self, states: &[i8]) -> alloc::vec::Vec<alloc::string::String> {
            __expected_tokens_from_states(states, core::marker::PhantomData::<()>)
        }

        #[inline]
        fn uses_error_recovery(&self) -> bool {
            false
        }

        #[inline]
        fn error_recovery_symbol(
            &self,
            recovery: __state_machine::ErrorRecovery<Self>,
        ) -> Self::Symbol {
            panic!("error recovery not enabled for this grammar")
        }

        fn reduce(
            &mut self,
            action: i8,
            start_location: Option<&Self::Location>,
            states: &mut alloc::vec::Vec<i8>,
            symbols: &mut alloc::vec::Vec<__state_machine::SymbolTriple<Self>>,
        ) -> Option<__state_machine::ParseResult<Self>> {
            __reduce(
                action,
                start_location,
                states,
                symbols,
                core::marker::PhantomData::<()>,
            )
        }

        fn simulate_reduce(&self, action: i8) -> __state_machine::SimulatedReduce<Self> {
            __simulate_reduce(action, core::marker::PhantomData::<()>)
        }
    }
    fn __token_to_integer<
    >(
        __token: &crate::lexer::Token<'input>,
        _: core::marker::PhantomData<()>,
    ) -> Option<usize>
    {
        #[warn(unused_variables)]
        match __token {
            Module if true => Some(0),
            Function if true => Some(1),
            If if true => Some(2),
            Else if true => Some(3),
            Let if true => Some(4),
            Assert if true => Some(5),
            Echo if true => Some(6),
            For if true => Some(7),
            Each if true => Some(8),
            True if true => Some(9),
            False if true => Some(10),
            Undef if true => Some(11),
            Le if true => Some(12),
            Ge if true => Some(13),
            Eq if true => Some(14),
            Neq if true => Some(15),
            And if true => Some(16),
            Or if true => Some(17),
            Lsh if true => Some(18),
            Rsh if true => Some(19),
            Include if true => Some(20),
            Use if true => Some(21),
            Lt if true => Some(22),
            Gt if true => Some(23),
            Plus if true => Some(24),
            Minus if true => Some(25),
            Star if true => Some(26),
            Slash if true => Some(27),
            Percent if true => Some(28),
            Caret if true => Some(29),
            EqSign if true => Some(30),
            Bang if true => Some(31),
            Tilde if true => Some(32),
            Amp if true => Some(33),
            Pipe if true => Some(34),
            Question if true => Some(35),
            Colon if true => Some(36),
            Dot if true => Some(37),
            Hash if true => Some(38),
            Semi if true => Some(39),
            LBrace if true => Some(40),
            RBrace if true => Some(41),
            LParen if true => Some(42),
            RParen if true => Some(43),
            LBracket if true => Some(44),
            RBracket if true => Some(45),
            Comma if true => Some(46),
            Eot if true => Some(47),
            Nbsp if true => Some(48),
            Bom if true => Some(49),
            Newline if true => Some(50),
            IntNumber if true => Some(51),
            FloatNumber if true => Some(52),
            HexNumber if true => Some(53),
            Ident if true => Some(54),
            String if true => Some(55),
            IncludePath if true => Some(56),
            _ => None,
        }
    }
    fn __token_to_symbol<
    >(
        __token_index: usize,
        __token: crate::lexer::Token<'input>,
        _: core::marker::PhantomData<()>,
    ) -> __Symbol<>
    {
        #[allow(clippy::manual_range_patterns)]match __token_index {
            0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 | 20 | 21 | 22 | 23 | 24 | 25 | 26 | 27 | 28 | 29 | 30 | 31 | 32 | 33 | 34 | 35 | 36 | 37 | 38 | 39 | 40 | 41 | 42 | 43 | 44 | 45 | 46 | 47 | 48 | 49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 => __Symbol::Variant0(__token),
            _ => unreachable!(),
        }
    }
    fn __simulate_reduce<
    >(
        __reduce_index: i8,
        _: core::marker::PhantomData<()>,
    ) -> __state_machine::SimulatedReduce<__StateMachine<>>
    {
        match __reduce_index {
            0 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 1,
                    nonterminal_produced: 0,
                }
            }
            1 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 0,
                }
            }
            2 => {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop: 2,
                    nonterminal_produced: 0,
                }
            }
            3 => __state_machine::SimulatedReduce::Accept,
            _ => panic!("invalid reduction index {__reduce_index}",)
        }
    }
    pub struct TermParser {
        _priv: (),
    }

    impl Default for TermParser { fn default() -> Self { Self::new() } }
    impl TermParser {
        pub fn new() -> TermParser {
            TermParser {
                _priv: (),
            }
        }

        #[allow(dead_code)]
        pub fn parse<
            __TOKEN: __ToTriple<>,
            __TOKENS: IntoIterator<Item=__TOKEN>,
        >(
            &self,
            __tokens0: __TOKENS,
        ) -> Result<i32, __lalrpop_util::ParseError<text_size::TextSize, crate::lexer::Token<'input>, lalrpop_util::ParseError<usize, crate::lexer::Token<'static>, &'static str>>>
        {
            let __tokens = __tokens0.into_iter();
            let mut __tokens = __tokens.map(|t| __ToTriple::to_triple(t));
            __state_machine::Parser::drive(
                __StateMachine {
                    __phantom: core::marker::PhantomData::<()>,
                },
                __tokens,
            )
        }
    }
    fn __accepts<
    >(
        __error_state: Option<i8>,
        __states: &[i8],
        __opt_integer: Option<usize>,
        _: core::marker::PhantomData<()>,
    ) -> bool
    {
        let mut __states = __states.to_vec();
        __states.extend(__error_state);
        loop {
            let mut __states_len = __states.len();
            let __top = __states[__states_len - 1];
            let __action = match __opt_integer {
                None => __EOF_ACTION[__top as usize],
                Some(__integer) => __action(__top, __integer),
            };
            if __action == 0 { return false; }
            if __action > 0 { return true; }
            let (__to_pop, __nt) = match __simulate_reduce(-(__action + 1), core::marker::PhantomData::<()>) {
                __state_machine::SimulatedReduce::Reduce {
                    states_to_pop, nonterminal_produced
                } => (states_to_pop, nonterminal_produced),
                __state_machine::SimulatedReduce::Accept => return true,
            };
            __states_len -= __to_pop;
            __states.truncate(__states_len);
            let __top = __states[__states_len - 1];
            let __next_state = __goto(__top, __nt);
            __states.push(__next_state);
        }
    }
    fn __reduce<
    >(
        __action: i8,
        __lookahead_start: Option<&text_size::TextSize>,
        __states: &mut alloc::vec::Vec<i8>,
        __symbols: &mut alloc::vec::Vec<(text_size::TextSize,__Symbol<>,text_size::TextSize)>,
        _: core::marker::PhantomData<()>,
    ) -> Option<Result<i32,__lalrpop_util::ParseError<text_size::TextSize, crate::lexer::Token<'input>, lalrpop_util::ParseError<usize, crate::lexer::Token<'static>, &'static str>>>>
    {
        let (__pop_states, __nonterminal) = match __action {
            0 => {
                __reduce0(__lookahead_start, __symbols, core::marker::PhantomData::<()>)
            }
            1 => {
                __reduce1(__lookahead_start, __symbols, core::marker::PhantomData::<()>)
            }
            2 => {
                __reduce2(__lookahead_start, __symbols, core::marker::PhantomData::<()>)
            }
            3 => {
                // __Term = Term => ActionFn(0);
                let __sym0 = __pop_Variant1(__symbols);
                let __start = __sym0.0;
                let __end = __sym0.2;
                let __nt = super::__action0::<>(__sym0);
                return Some(Ok(__nt));
            }
            _ => panic!("invalid action code {__action}")
        };
        let __states_len = __states.len();
        __states.truncate(__states_len - __pop_states);
        let __state = *__states.last().unwrap();
        let __next_state = __goto(__state, __nonterminal);
        __states.push(__next_state);
        None
    }
    #[inline(never)]
    fn __symbol_type_mismatch() -> ! {
        panic!("symbol type mismatch")
    }
    fn __pop_Variant0<
    >(
        __symbols: &mut alloc::vec::Vec<(text_size::TextSize,__Symbol<>,text_size::TextSize)>
    ) -> (text_size::TextSize, crate::lexer::Token<'input>, text_size::TextSize)
     {
        match __symbols.pop() {
            Some((__l, __Symbol::Variant0(__v), __r)) => (__l, __v, __r),
            _ => __symbol_type_mismatch()
        }
    }
    fn __pop_Variant1<
    >(
        __symbols: &mut alloc::vec::Vec<(text_size::TextSize,__Symbol<>,text_size::TextSize)>
    ) -> (text_size::TextSize, i32, text_size::TextSize)
     {
        match __symbols.pop() {
            Some((__l, __Symbol::Variant1(__v), __r)) => (__l, __v, __r),
            _ => __symbol_type_mismatch()
        }
    }
    fn __reduce0<
    >(
        __lookahead_start: Option<&text_size::TextSize>,
        __symbols: &mut alloc::vec::Vec<(text_size::TextSize,__Symbol<>,text_size::TextSize)>,
        _: core::marker::PhantomData<()>,
    ) -> (usize, usize)
    {
        // Term = "number" => ActionFn(1);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0;
        let __end = __sym0.2;
        let __nt = super::__action1::<>(__sym0);
        __symbols.push((__start, __Symbol::Variant1(__nt), __end));
        (1, 0)
    }
    fn __reduce1<
    >(
        __lookahead_start: Option<&text_size::TextSize>,
        __symbols: &mut alloc::vec::Vec<(text_size::TextSize,__Symbol<>,text_size::TextSize)>,
        _: core::marker::PhantomData<()>,
    ) -> (usize, usize)
    {
        // Term = "+", "number" => ActionFn(2);
        assert!(__symbols.len() >= 2);
        let __sym1 = __pop_Variant0(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0;
        let __end = __sym1.2;
        let __nt = super::__action2::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant1(__nt), __end));
        (2, 0)
    }
    fn __reduce2<
    >(
        __lookahead_start: Option<&text_size::TextSize>,
        __symbols: &mut alloc::vec::Vec<(text_size::TextSize,__Symbol<>,text_size::TextSize)>,
        _: core::marker::PhantomData<()>,
    ) -> (usize, usize)
    {
        // Term = "-", "number" => ActionFn(3);
        assert!(__symbols.len() >= 2);
        let __sym1 = __pop_Variant0(__symbols);
        let __sym0 = __pop_Variant0(__symbols);
        let __start = __sym0.0;
        let __end = __sym1.2;
        let __nt = super::__action3::<>(__sym0, __sym1);
        __symbols.push((__start, __Symbol::Variant1(__nt), __end));
        (2, 0)
    }
}
#[allow(unused_imports)]
pub use self::__parse__Term::TermParser;

#[allow(clippy::too_many_arguments, clippy::needless_lifetimes, clippy::just_underscores_and_digits)]
fn __action0<
>(
    (_, __0, _): (text_size::TextSize, i32, text_size::TextSize),
) -> i32
{
    __0
}

#[allow(clippy::too_many_arguments, clippy::needless_lifetimes, clippy::just_underscores_and_digits)]
fn __action1<
>(
    (_, __0, _): (text_size::TextSize, crate::lexer::Token<'input>, text_size::TextSize),
) -> i32
{
    1
}

#[allow(clippy::too_many_arguments, clippy::needless_lifetimes, clippy::just_underscores_and_digits)]
fn __action2<
>(
    (_, __0, _): (text_size::TextSize, crate::lexer::Token<'input>, text_size::TextSize),
    (_, __1, _): (text_size::TextSize, crate::lexer::Token<'input>, text_size::TextSize),
) -> i32
{
    1 + 1
}

#[allow(clippy::too_many_arguments, clippy::needless_lifetimes, clippy::just_underscores_and_digits)]
fn __action3<
>(
    (_, __0, _): (text_size::TextSize, crate::lexer::Token<'input>, text_size::TextSize),
    (_, __1, _): (text_size::TextSize, crate::lexer::Token<'input>, text_size::TextSize),
) -> i32
{
    1 - 1
}

#[allow(clippy::type_complexity, dead_code)]
pub trait __ToTriple<>
{
    fn to_triple(self) -> Result<(text_size::TextSize,crate::lexer::Token<'input>,text_size::TextSize), __lalrpop_util::ParseError<text_size::TextSize, crate::lexer::Token<'input>, lalrpop_util::ParseError<usize, crate::lexer::Token<'static>, &'static str>>>;
}

impl<> __ToTriple<> for (text_size::TextSize, crate::lexer::Token<'input>, text_size::TextSize)
{
    fn to_triple(self) -> Result<(text_size::TextSize,crate::lexer::Token<'input>,text_size::TextSize), __lalrpop_util::ParseError<text_size::TextSize, crate::lexer::Token<'input>, lalrpop_util::ParseError<usize, crate::lexer::Token<'static>, &'static str>>> {
        Ok(self)
    }
}
impl<> __ToTriple<> for Result<(text_size::TextSize, crate::lexer::Token<'input>, text_size::TextSize), lalrpop_util::ParseError<usize, crate::lexer::Token<'static>, &'static str>>
{
    fn to_triple(self) -> Result<(text_size::TextSize,crate::lexer::Token<'input>,text_size::TextSize), __lalrpop_util::ParseError<text_size::TextSize, crate::lexer::Token<'input>, lalrpop_util::ParseError<usize, crate::lexer::Token<'static>, &'static str>>> {
        self.map_err(|error| __lalrpop_util::ParseError::User { error })
    }
}
