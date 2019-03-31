use std::fmt;

use byteorder::{LittleEndian, WriteBytesExt};
use nom::types::CompleteStr;

use assembler::comment_parsers::comment;
use assembler::label_parsers::label_declaration;
use assembler::opcode_parsers::*;
use assembler::operand_parsers::operand;
use assembler::{SymbolTable, Token};
use instruction;

const MAX_I16: i32 = 32768;
const MIN_I16: i32 = -32768;

#[derive(Debug, PartialEq)]
pub struct AssemblerInstruction {
    pub opcode: Option<Token>,
    pub label: Option<Token>,
    pub directive: Option<Token>,
    pub operand1: Option<Token>,
    pub operand2: Option<Token>,
    pub operand3: Option<Token>,
}

impl AssemblerInstruction {
    pub fn to_bytes(&self, symbols: &SymbolTable) -> Vec<u8> {
        let mut results: Vec<u8> = vec![];
        if let Some(ref token) = self.opcode {
            match token {
                Token::Op { code } => match code {
                    _ => {
                        let b: u8 = (*code).into();
                        results.push(b);
                    }
                },
                _ => {
                    println!("Non-opcode found in opcode field");
                }
            }
        }
        for operand in &[&self.operand1, &self.operand2, &self.operand3] {
            if let Some(token) = operand {
                AssemblerInstruction::extract_operand(token, &mut results, symbols);
            }
        }
        while results.len() < 4 {
            results.push(0);
        }

        results
    }

    pub fn is_label(&self) -> bool {
        self.label.is_some()
    }

    pub fn is_opcode(&self) -> bool {
        self.opcode.is_some()
    }

    pub fn is_integer_needs_splitting(&self) -> bool {
        if let Some(ref op) = self.opcode {
            match op {
                Token::Op { code } => match code {
                    instruction::Opcode::LOAD => {
                        if let Some(ref first_half) = self.operand2 {
                            match first_half {
                                Token::IntegerOperand { ref value } => {
                                    if *value > MAX_I16 || *value < MIN_I16 {
                                        return true;
                                    }
                                    return false;
                                }
                                _ => {
                                    return false;
                                }
                            }
                        }
                        return true;
                    }
                    _ => {
                        return false;
                    }
                },
                _ => {
                    return false;
                }
            }
        }
        false
    }

    pub fn is_directive(&self) -> bool {
        self.directive.is_some()
    }

    pub fn get_integer_value(&self) -> Option<i16> {
        if let Some(ref operand) = self.operand2 {
            match operand {
                Token::IntegerOperand { ref value } => return Some(*value as i16),
                _ => return None,
            }
        }
        None
    }

    pub fn get_register_number(&self) -> Option<u8> {
        match self.operand1 {
            Some(ref reg_token) => match reg_token {
                Token::Register { ref reg_num } => Some(reg_num.clone()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn set_opernand_two(&mut self, t: Token) {
        self.operand2 = Some(t)
    }

    pub fn set_operand_three(&mut self, t: Token) {
        self.operand3 = Some(t)
    }

    /// Checks if the AssemblyInstruction has any operands at all
    pub fn has_operands(&self) -> bool {
        self.operand1.is_some() || self.operand2.is_some() || self.operand3.is_some()
    }

    pub fn get_directive_name(&self) -> Option<String> {
        match &self.directive {
            Some(d) => match d {
                Token::Directive { name } => Some(name.to_string()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn get_string_constant(&self) -> Option<String> {
        match &self.operand1 {
            Some(d) => match d {
                Token::IrString { name } => Some(name.to_string()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn get_i32_constant(&self) -> Option<i32> {
        match &self.operand1 {
            Some(d) => match d {
                Token::IntegerOperand { value } => Some(*value),
                _ => None,
            },
            None => None,
        }
    }

    pub fn get_label_name(&self) -> Option<String> {
        match &self.label {
            Some(l) => match l {
                Token::LabelDeclaration { name } => Some(name.clone()),
                _ => None,
            },
            None => None,
        }
    }

    fn extract_operand(t: &Token, results: &mut Vec<u8>, symbols: &SymbolTable) {
        match t {
            Token::Register { reg_num } => {
                results.push(*reg_num);
            }
            Token::IntegerOperand { value } => {
                let mut wtr = vec![];
                wtr.write_i16::<LittleEndian>(*value as i16).unwrap();
                results.push(wtr[1]);
                results.push(wtr[0]);
            }
            Token::LabelUsage { name } => {
                if let Some(value) = symbols.symbol_value(name) {
                    let mut wtr = vec![];
                    wtr.write_u32::<LittleEndian>(value).unwrap();
                    results.push(wtr[1]);
                    results.push(wtr[0]);
                } else {
                    error!("No value found for {:?}", name);
                }
            }
            _ => {
                error!("Opcode found in operand field: {:#?}", t);
            }
        };
    }
}

impl fmt::Display for AssemblerInstruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(Label: {:?} Opcode: {:?} Directive: {:?} Operand #1: {:?} Operand #2: {:?} Operand #3: {:?})", self.label, self.opcode, self.directive, self.operand1, self.operand2, self.operand3)
    }
}

named!(instruction_combined<CompleteStr, AssemblerInstruction>,
    do_parse!(
        opt!(comment) >>
        l: opt!(label_declaration) >>
        o: opcode >>
        o1: opt!(operand) >>
        o2: opt!(operand) >>
        o3: opt!(operand) >>
        opt!(comment) >>
        (
            {
            AssemblerInstruction{
                opcode: Some(o),
                label: l,
                directive: None,
                operand1: o1,
                operand2: o2,
                operand3: o3,
            }
            }
        )
    )
);

/// Will try to parse out any of the Instruction forms
named!(pub instruction<CompleteStr, AssemblerInstruction>,
    do_parse!(
        ins: alt!(
            instruction_combined
        ) >>
        (
            ins
        )
    )
);

#[cfg(test)]
mod tests {
    use super::*;
    use instruction::Opcode;

    #[test]
    fn test_parse_instruction_form_one() {
        let result = instruction_combined(CompleteStr("load $0 #100\n"));
        assert_eq!(
            result,
            Ok((
                CompleteStr(""),
                AssemblerInstruction {
                    opcode: Some(Token::Op { code: Opcode::LOAD }),
                    label: None,
                    directive: None,
                    operand1: Some(Token::Register { reg_num: 0 }),
                    operand2: Some(Token::IntegerOperand { value: 100 }),
                    operand3: None
                }
            ))
        );
    }

    #[test]
    fn test_parse_instruction_form_one_with_label() {
        let result = instruction_combined(CompleteStr("load $0 @test1\n"));
        assert_eq!(
            result,
            Ok((
                CompleteStr(""),
                AssemblerInstruction {
                    opcode: Some(Token::Op { code: Opcode::LOAD }),
                    label: None,
                    directive: None,
                    operand1: Some(Token::Register { reg_num: 0 }),
                    operand2: Some(Token::LabelUsage {
                        name: "test1".to_string()
                    }),
                    operand3: None
                }
            ))
        );
    }

    #[test]
    fn test_parse_instruction_form_two() {
        let result = instruction_combined(CompleteStr("hlt"));
        assert_eq!(
            result,
            Ok((
                CompleteStr(""),
                AssemblerInstruction {
                    opcode: Some(Token::Op { code: Opcode::HLT }),
                    label: None,
                    directive: None,
                    operand1: None,
                    operand2: None,
                    operand3: None
                }
            ))
        );
    }

    #[test]
    fn test_parse_instruction_form_three() {
        let result = instruction_combined(CompleteStr("add $0 $1 $2\n"));
        assert_eq!(
            result,
            Ok((
                CompleteStr(""),
                AssemblerInstruction {
                    opcode: Some(Token::Op { code: Opcode::ADD }),
                    label: None,
                    directive: None,
                    operand1: Some(Token::Register { reg_num: 0 }),
                    operand2: Some(Token::Register { reg_num: 1 }),
                    operand3: Some(Token::Register { reg_num: 2 }),
                }
            ))
        );
    }

    #[test]
    fn test_parse_instruction_with_comment_one() {
        let result = instruction_combined(CompleteStr("; this is a test\nadd $0 $1 $2\n"));
        assert_eq!(
            result,
            Ok((
                CompleteStr(""),
                AssemblerInstruction {
                    opcode: Some(Token::Op { code: Opcode::ADD }),
                    label: None,
                    directive: None,
                    operand1: Some(Token::Register { reg_num: 0 }),
                    operand2: Some(Token::Register { reg_num: 1 }),
                    operand3: Some(Token::Register { reg_num: 2 }),
                }
            ))
        );
    }

    #[test]
    fn test_parse_instruction_with_comment_two() {
        let result = instruction_combined(CompleteStr("add $0 $1 $2 ; this is a test\n"));
        assert_eq!(
            result,
            Ok((
                CompleteStr(""),
                AssemblerInstruction {
                    opcode: Some(Token::Op { code: Opcode::ADD }),
                    label: None,
                    directive: None,
                    operand1: Some(Token::Register { reg_num: 0 }),
                    operand2: Some(Token::Register { reg_num: 1 }),
                    operand3: Some(Token::Register { reg_num: 2 }),
                }
            ))
        );
    }

    #[test]
    fn test_parse_cloop() {
        let result = instruction_combined(CompleteStr("cloop #10\n"));
        assert_eq!(
            result,
            Ok((
                CompleteStr(""),
                AssemblerInstruction {
                    opcode: Some(Token::Op {
                        code: Opcode::CLOOP
                    }),
                    label: None,
                    directive: None,
                    operand1: Some(Token::IntegerOperand { value: 10 }),
                    operand2: None,
                    operand3: None
                }
            ))
        );
    }
}
