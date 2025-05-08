use std::io::{BufRead, Write};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BfInterpreter<I, O> {
    instructions: Vec<BfInstruction>,
    instruction_pointer: usize,
    jump_memo: Vec<usize>,
    tape: Vec<u8>,
    tape_pointer: usize,
    input: I,
    output: O,
}

impl<I, O> BfInterpreter<I, O>
where
    I: BufRead,
    O: Write,
{
    pub fn new(source: &str, input: I, output: O) -> Result<Self, BfError> {
        let mut instructions = vec![];
        let mut loop_stack = vec![];
        let mut jump_memo = vec![];
        for code in source.chars() {
            match code {
                '>' => instructions.push(BfInstruction::GoRight),
                '<' => instructions.push(BfInstruction::GoLeft),
                '+' => instructions.push(BfInstruction::Increment),
                '-' => instructions.push(BfInstruction::Decrement),
                ',' => instructions.push(BfInstruction::GetInput),
                '.' => instructions.push(BfInstruction::PutOutput),
                '[' => {
                    loop_stack.push(instructions.len());
                    instructions.push(BfInstruction::LoopStart);
                }
                ']' => {
                    let ending = instructions.len();
                    let Some(beginning) = loop_stack.pop() else {
                        return Err(BfError::LoopNotStarted);
                    };
                    if jump_memo.len() < ending {
                        jump_memo.resize(ending + 1, 0);
                    }
                    jump_memo[beginning] = ending;
                    jump_memo[ending] = beginning;
                    instructions.push(BfInstruction::LoopEnd);
                }
                _ => {}
            }
        }
        if !loop_stack.is_empty() {
            return Err(BfError::LoopNotEnded);
        }
        Ok(Self {
            instructions,
            instruction_pointer: 0,
            jump_memo,
            tape: vec![0],
            tape_pointer: 0,
            input,
            output,
        })
    }

    pub fn is_end(&self) -> bool {
        self.instruction_pointer >= self.instructions.len()
    }

    pub fn head_value(&self) -> u8 {
        self.tape[self.tape_pointer]
    }

    fn head_value_mut(&mut self) -> &mut u8 {
        &mut self.tape[self.tape_pointer]
    }

    pub fn current_instruction(&self) -> &BfInstruction {
        &self.instructions[self.instruction_pointer]
    }

    pub fn step(&mut self) -> Result<(), BfError> {
        match self.current_instruction() {
            BfInstruction::GoRight => {
                self.tape_pointer += 1;
                if self.tape_pointer >= self.tape.len() {
                    self.tape.push(0);
                }
            }
            BfInstruction::GoLeft => {
                if self.tape_pointer == 0 {
                    return Err(BfError::SeekOverLeftmost);
                }
                self.tape_pointer -= 1;
            }
            BfInstruction::Increment => {
                *self.head_value_mut() += 1;
            }
            BfInstruction::Decrement => {
                *self.head_value_mut() -= 1;
            }
            BfInstruction::GetInput => {
                let buf = self.input.fill_buf()?;
                if buf.is_empty() {
                    return Err(BfError::LackOfInput);
                }
                *self.head_value_mut() = buf[0];
                self.input.consume(1);
            }
            BfInstruction::PutOutput => {
                self.output.write(&[self.head_value()])?;
            }
            BfInstruction::LoopStart => {
                if self.head_value() == 0 {
                    self.instruction_pointer = self.jump_memo[self.instruction_pointer];
                }
            }
            BfInstruction::LoopEnd => {
                if self.head_value() != 0 {
                    self.instruction_pointer = self.jump_memo[self.instruction_pointer];
                }
            }
        }
        self.instruction_pointer += 1;
        Ok(())
    }

    pub fn execute(mut self) -> Result<(), BfError> {
        while !self.is_end() {
            self.step()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BfInstruction {
    GoRight,
    GoLeft,
    Increment,
    Decrement,
    GetInput,
    PutOutput,
    LoopStart,
    LoopEnd,
}

#[derive(Debug, Error)]
pub enum BfError {
    #[error("syntax error: starting loop `[` unmatched")]
    LoopNotStarted,
    #[error("syntax error: ending loop `]` unmatched")]
    LoopNotEnded,
    #[error("cannot seek over leftmost of tape")]
    SeekOverLeftmost,
    #[error("lack of input")]
    LackOfInput,
    #[error("input read error")]
    ReadError(#[from] std::io::Error),
}

#[test]
fn test_echo() -> anyhow::Result<()> {
    let input = std::io::BufReader::new(&[1, 4, 2, 3, 5, 2, 3, 0][..]);
    let mut output = vec![];
    let interpreter = BfInterpreter::new(",[.,]", input, &mut output)?;
    interpreter.execute()?;
    assert_eq!(output, [1, 4, 2, 3, 5, 2, 3]);
    Ok(())
}

#[test]
fn test_reverse() -> anyhow::Result<()> {
    let input = std::io::BufReader::new(&[1, 4, 2, 3, 5, 2, 3, 0][..]);
    let mut output = vec![];
    let interpreter = BfInterpreter::new(">,[>,]<[.<]", input, &mut output)?;
    interpreter.execute()?;
    assert_eq!(output, [3, 2, 5, 3, 2, 4, 1]);
    Ok(())
}

#[test]
fn test_hello_world() -> anyhow::Result<()> {
    let input = std::io::BufReader::new(&[][..]);
    let mut output = vec![];
    let interpreter = BfInterpreter::new(
        "++++++++++[>+++++++>++++++++++>+++>++++<
<<<-]>++.>+.+++++++..+++.>>++++.<++.<+++
+++++.--------.+++.------.--------.>+.",
        input,
        &mut output,
    )?;
    interpreter.execute()?;
    assert_eq!(output, b"Hello, world!");
    Ok(())
}

#[test]
fn test_sum_n() -> anyhow::Result<()> {
    let input = std::io::BufReader::new(&[3][..]);
    let mut output = vec![];
    let interpreter = BfInterpreter::new(
        ",[[->>+>+<<<]>>>[-<<<+>>>]<[-<+>]<<-]>.",
        input,
        &mut output,
    )?;
    interpreter.execute()?;
    assert_eq!(output, [6]);
    Ok(())
}

#[test]
fn test_not_opening_loop() -> anyhow::Result<()> {
    let input = std::io::BufReader::new(&[][..]);
    let mut output = vec![];
    let res = BfInterpreter::new("]", input, &mut output);
    let err = res.expect_err("must occur syntax error");
    assert!(matches!(err, BfError::LoopNotStarted));
    Ok(())
}

#[test]
fn test_not_closing_loop() -> anyhow::Result<()> {
    let input = std::io::BufReader::new(&[][..]);
    let mut output = vec![];
    let res = BfInterpreter::new("[", input, &mut output);
    let err = res.expect_err("must occur syntax error");
    assert!(matches!(err, BfError::LoopNotEnded));
    Ok(())
}
