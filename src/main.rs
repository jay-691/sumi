use std::{io::{BufReader, self, BufRead, BufWriter, Write}, fs};
use ethabi::ParamType;
use hex::ToHex;
use serde::Serialize;

use tinytemplate::{TinyTemplate, format_unescaped};
use clap::Parser;
use convert_case::{Case, Casing};
use itertools::Itertools;
use sha3::{Digest, Keccak256};

#[derive(Parser, Debug)]
struct Args {
    /// Input filename or stdin if empty
    #[arg(long, short)]
    input: Option<String>,

    /// Output filename or stdout if empty
    #[arg(long, short)]
    output: Option<String>,

    /// Ink module name to generate
    #[arg(long, short)]
    module_name: String,

    /// EVM ID to use in module
    #[arg(long, short, default_value = "0x0F")]
    evm_id: String,
}

static MODULE_TEMPLATE: &'static str = r#"
//! This file was autogenerated by Sumi
#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;
pub use self::{name}::\{
    {name | capitalize},
    {name | capitalize}Ref,
    FixedBytes,
    H160,
    U256,
};

/// EVM ID from runtime
const EVM_ID: u8 = {evm_id};

/// The EVM ERC20 delegation contract.
#[ink::contract(env = xvm_environment::XvmDefaultEnvironment)]
mod {name} \{
{{ for function in functions }}
    // Selector for `{function.selector}`
    const {function.name | upper_snake}_SELECTOR: [u8; 4] = hex!["{function.selector_hash}"];
{{ endfor }}

    use ethabi::Token;
    use hex_literal::hex;
    use ink_prelude::vec::Vec;
    use ink_storage::traits::\{StorageLayout, SpreadLayout};
    use scale::\{Encode, Decode};
    use scale_info::TypeInfo;

    #[ink(storage)]
    pub struct {name | capitalize} \{
        evm_address: H160,
    }

    impl {name | capitalize} \{
        /// Create new abstraction from given contract address.
        #[ink(constructor)]
        pub fn new(evm_address: H160) -> Self \{
            Self \{ evm_address }
        }

{{ for function in functions }}
        /// Send `{function.name}` call to contract
        #[ink(message)]
        pub fn {function.name | snake}(&mut self, {{ for input in function.inputs }}{input.name}: {input.rust_type}{{ if not @last }}, {{ endif }}{{ endfor }}) -> {function.output} \{
            let encoded_input = Self::{function.name | snake}_encode({{ for input in function.inputs }}{input.name}{{ if not @last }}, {{ endif }}{{ endfor }});
            self.env()
                .extension()
                .xvm_call(
                    super::EVM_ID,
                    Vec::from(self.evm_address.0.as_ref()),
                    encoded_input,
                )
                .is_ok()
        }

        fn {function.name | snake}_encode({{ for input in function.inputs }}{input.name}: {input.rust_type}{{ if not @last }}, {{ endif }}{{ endfor }}) -> Vec<u8> \{
            let mut encoded = {function.name | upper_snake}_SELECTOR.to_vec();
            let input = [
                {{ for input in function.inputs }}{input.name}.tokenize(){{ if not @last }},
                {{ endif }}{{ endfor }}
            ];

            encoded.extend(&ethabi::encode(&input));
            encoded
        }
{{ endfor }}
    }

    /// Custom wrapper to make `H160` scale-encodable
    #[derive(Debug, Encode, Decode, TypeInfo, StorageLayout, SpreadLayout)]
    pub struct H160([u8; 20]);

    /// Custom wrapper to make `U256` scale-encodable
    #[derive(Debug, Encode, Decode, TypeInfo)]
    pub struct U256([u8; 32]);

    impl From<[u8; 20]> for H160 \{
        fn from(other: [u8; 20]) -> Self \{
            H160(other)
        }
    }

    impl From<ethabi::ethereum_types::H160> for H160 \{
        fn from(other: ethabi::ethereum_types::H160) -> Self \{
            H160(other.to_fixed_bytes())
        }
    }

    impl Into<ethabi::ethereum_types::H160> for H160 \{
        fn into(self) -> ethabi::ethereum_types::H160 \{
            ethabi::ethereum_types::H160::from(self.0)
        }
    }

    impl From<[u8; 32]> for U256 \{
        fn from(other: [u8; 32]) -> Self \{
            U256(other)
        }
    }

    impl From<ethabi::ethereum_types::U256> for U256 \{
        fn from(other: ethabi::ethereum_types::U256) -> Self \{
            U256(other.into())
        }
    }

    impl Into<ethabi::ethereum_types::U256> for U256 \{
        fn into(self) -> ethabi::ethereum_types::U256 \{
            ethabi::ethereum_types::U256::from(self.0)
        }
    }

    /// Helper trait used to convert Rust types to their serializable `Token` counterparts.
    /// Should be 100% inlined and therefore should not negatively affect smart contract size.
    trait Tokenize \{
        fn tokenize(self) -> Token;
    }

    impl<T: Tokenize, const N: usize> Tokenize for [T; N] \{
        fn tokenize(self) -> Token \{
            Token::FixedArray(self.into_iter().map(Tokenize::tokenize).collect())
        }
    }

    impl<T: Tokenize> Tokenize for Vec<T> \{
        fn tokenize(self) -> Token \{
            Token::Array(self.into_iter().map(Tokenize::tokenize).collect())
        }
    }

    /// Rust currently lacks specialization, thus overlapping trait implementations are forbidden.
    /// We use this newtype wrapper to provide custom tokenize implementation for byte arrays.
    pub struct FixedBytes<const N: usize>(pub [u8; N]);

    impl<const N: usize> From<[u8; N]> for FixedBytes<N> \{
        fn from(other: [u8; N]) -> Self \{
            FixedBytes(other)
        }
    }

    impl<const N: usize> Into<[u8; N]> for FixedBytes<N> \{
        fn into(self) -> [u8; N] \{
            self.0
        }
    }

    impl<const N: usize> Tokenize for FixedBytes<N> \{
        fn tokenize(self) -> Token \{
            Token::FixedBytes(Vec::from(self.0))
        }
    }

    macro_rules! tokenize_tuple \{
        ($($i:ident),+) => \{
            impl<$($i: Tokenize,)+> Tokenize for ($($i,)+) \{
                fn tokenize(self) -> Token \{
                    #[allow(non_snake_case)]
                    let ($($i,)+) = self;

                    Token::Tuple(vec![$($i.tokenize(),)+])
                }
            }
        };
    }

    tokenize_tuple!(A);
    tokenize_tuple!(A, B);
    tokenize_tuple!(A, B, C);
    tokenize_tuple!(A, B, C, D);
    tokenize_tuple!(A, B, C, D, E);
    tokenize_tuple!(A, B, C, D, E, F);
    tokenize_tuple!(A, B, C, D, E, F, G);
    tokenize_tuple!(A, B, C, D, E, F, G, H);

    macro_rules! tokenize_ints \{
        (unsigned: $($t:ty),+) => \{
            $(
                impl Tokenize for $t \{
                    fn tokenize(self) -> Token \{
                        Token::Uint(self.into())
                    }
                }
            )+
        };

        (signed: $($t:ty),+) => \{
            $(
                impl Tokenize for $t \{
                    fn tokenize(self) -> Token \{
                        Token::Int(self.into())
                    }
                }
            )+
        };
    }

    tokenize_ints!(signed: i8, i16, i32, i64, i128);
    tokenize_ints!(unsigned: u8, u16, u32, u64, u128);

    impl Tokenize for H160 \{
        fn tokenize(self) -> Token \{
            Token::Address(self.0.into())
        }
    }

    impl Tokenize for bool \{
        fn tokenize(self) -> Token \{
            Token::Bool(self)
        }
    }

    impl Tokenize for String \{
        fn tokenize(self) -> Token \{
            Token::String(self)
        }
    }

    impl Tokenize for U256 \{
        fn tokenize(self) -> Token \{
            Token::Uint(ethabi::ethereum_types::U256::from(self.0))
        }
    }
}
"#;

#[derive(Serialize)]
struct Input {
    name: String,

    // Type came from metadata
    evm_type: String,

    // Equivalent type to use in ink! code
    rust_type: String,
}

#[derive(Serialize)]
struct Function {
    name: String,
    inputs: Vec<Input>,
    output: String,
    selector: String,
    selector_hash: String,
}

#[derive(Serialize)]
struct Module {
    name: String,
    evm_id: String,
    functions: Vec<Function>,
}

fn convert_type(ty: &ParamType) -> String {
    match ty {
        ParamType::Bool => "bool".to_owned(),
        ParamType::Address => "H160".to_owned(),
        ParamType::Array(inner) => format!("Vec<{}>", convert_type(inner)),
        ParamType::FixedArray(inner, size) => format!("[{}; {}]", convert_type(inner), size),
        ParamType::Tuple(inner) => format!("({})", inner.iter().map(convert_type).join(", ")),
        ParamType::FixedBytes(size) => format!("FixedBytes<{}>", size),
        ParamType::Bytes => "Vec<u8>".to_owned(),
        ParamType::String => "String".to_owned(),

        ParamType::Int(size) => match size {
            8 => "i8",
            16 => "i16",
            32 => "i32",
            64 => "i64",
            128 => "i128",

            _ => "I256",
        }.to_owned(),

        ParamType::Uint(size) => match size {
            8 => "u8",
            16 => "u16",
            32 => "u32",
            64 => "u64",
            128 => "u128",

            _ => "U256",
        }.to_owned(),
    }
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    let mut reader: Box<dyn BufRead> = match args.input {
        Some(filename) => Box::new(BufReader::new(fs::File::open(filename).map_err(|e| e.to_string())?)),
        None => Box::new(BufReader::new(io::stdin())),
    };

    let mut writer: Box<dyn Write> = match args.output {
        Some(filename) => Box::new(BufWriter::new(fs::File::create(filename).map_err(|e| e.to_string())?)),
        None => Box::new(BufWriter::new(io::stdout())),
    };

    let mut buf = String::new();
    reader.read_to_string(&mut buf).map_err(|e| e.to_string())?;

    let parsed = json::parse(&buf).map_err(|e| e.to_string())?;

    let mut template = TinyTemplate::new();
    template.set_default_formatter(&format_unescaped);

    template.add_template("module", MODULE_TEMPLATE).map_err(|e| e.to_string())?;

    template.add_formatter("snake", |value, buf| match value {
        serde_json::Value::String(s) => { *buf += &s.to_case(Case::Snake); Ok(()) },
        _ => Err(tinytemplate::error::Error::GenericError { msg: "string value expected".to_owned() }),
    });

    template.add_formatter("upper_snake", |value, buf| match value {
        serde_json::Value::String(s) => { *buf += &s.to_case(Case::UpperSnake); Ok(()) },
        _ => Err(tinytemplate::error::Error::GenericError { msg: "string value expected".to_owned() }),
    });

    template.add_formatter("capitalize", |value, buf| match value {
        serde_json::Value::String(s) => {
            let (head, tail) = s.split_at(1);

            *buf += &head.to_uppercase();
            *buf += tail;

            Ok(())
        },
        _ => Err(tinytemplate::error::Error::GenericError { msg: "string value expected".to_owned() }),
    });

    template.add_formatter("convert_type", |value, buf| match value {
        serde_json::Value::String(raw_type) => {
            let param_type = ethabi::param_type::Reader::read(raw_type).unwrap();
            let converted = convert_type(&param_type);

            buf.push_str(&converted);
            Ok(())
        },

        _ => Err(tinytemplate::error::Error::GenericError { msg: "string value expected".to_owned() }),
    });

    let functions: Vec<_> = parsed
        .members()
        .filter(|item| item["type"] == "function" )
        .filter(|item| item["stateMutability"] != "view" )
        .filter(|item| item["outputs"].members().all(|output| output["type"] == "bool"))
        .map(|function| {
            let function_name = function["name"].to_string();

            let inputs: Vec<_> = function["inputs"].members().map(|m| {
                let raw_type = m["type"].as_str().unwrap();
                let param_type = ethabi::param_type::Reader::read(raw_type).unwrap();
                let converted = convert_type(&param_type);

                Input {
                    name: m["name"].to_string(),
                    evm_type: raw_type.to_string(),
                    rust_type: converted,
                }
            }).collect();

            // let outputs: String = function["outputs"].members().map(|m| format!("{}: {}, ", m["name"], m["type"])).collect();

            let selector = format!("{name}({args})",
                name = function_name,
                args = inputs.iter().map(|input| input.evm_type.as_str()).join(","),
            );

            let mut hasher = Keccak256::new();
            hasher.update(selector.as_bytes());
            let selector_hash: &[u8] = &hasher.finalize();
            let selector_hash: [u8; 4] = selector_hash[0..=3].try_into().unwrap();

            Function {
                name: function_name,
                inputs,
                output: "bool".to_owned(),
                selector,
                selector_hash: selector_hash.encode_hex(),
            }
        })
        .collect();

    let module = Module {
        name: args.module_name,
        evm_id: args.evm_id,
        functions,
    };

    let rendered = template.render("module", &module).map_err(|e| e.to_string())?;
    write!(writer, "{}\n", rendered).map_err(|e| e.to_string())?;

    Ok(())
}
