use crate::frontend::ast::*;
use crate::frontend::symbol::*;
use crate::frontend::token::Ranged;
use std::fs::*;
use std::io::Write;
use std::path::Path;

trait Indent {
    fn indent(&self, level: usize) -> String;
}

impl Indent for &str {
    fn indent(&self, level: usize) -> String {
        let mut result = String::new();
        for l in self.lines() {
            if l.is_empty() {
                continue;
            }
            result.push_str(&"    ".repeat(level));
            result.push_str(l);
            result.push('\n');
        }
        if !self.ends_with('\n') {
            result.pop();
        }
        result
    }
}

impl Indent for String {
    fn indent(&self, level: usize) -> String {
        self.as_str().indent(level)
    }
}

trait Generator {
    fn pattern(&self, level: usize) -> String;
    fn error(&self, level: usize) -> String;
}

impl Generator for std::collections::BTreeSet<Symbol> {
    fn pattern(&self, level: usize) -> String {
        if !self.is_empty() {
            let symbols: Vec<_> = self.iter().map(|s| format!("pattern_{}!()", s)).collect();
            symbols.join(&format!("\n{}| ", "    ".repeat(level)))
        } else {
            "pattern_EOF!()".to_string()
        }
    }
    fn error(&self, level: usize) -> String {
        if !self.is_empty() {
            let symbols: Vec<_> = self.iter().map(|s| format!("default_{}!()", s)).collect();
            symbols.join(&format!(",\n{}", "    ".repeat(level)))
        } else {
            "default_EOF!()".to_string()
        }
    }
}

pub struct RustOutput {}

impl RustOutput {
    pub fn create_parser(module: &Module, path: &Path, version: &str) -> std::io::Result<()> {
        let mut file = File::create(path.join("parser.rs"))?;
        file.write_all(
            format!(
                "// generated by lelwel {}\n\n\
                #![allow(non_snake_case)]\n\
                #![allow(unused_variables)]\n\n\
                use super::token::*;\n\n",
                version
            )
            .as_bytes(),
        )?;
        Self::output_preamble(module, &mut file)?;
        Self::output_tokens(module, &mut file)?;
        Self::output_patterns(module, &mut file)?;
        Self::output_defaults(module, &mut file)?;
        Self::output_error(module, &mut file)?;
        Self::output_check_limit(module, &mut file)?;
        Self::output_consumes(module, &mut file)?;
        Self::output_display(module, &mut file)?;
        Self::output_parser(module, &mut file)
    }

    pub fn create_llw_skel(path: &Path) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(
            b"start:\
            \n  #1\
            \n;\
            \n\
            \nstart#1 {\
            \n    Ok(())\
            \n}",
        )
    }

    pub fn create_token(path: &Path) -> std::io::Result<()> {
        let mut file = File::create(path.join("token.rs"))?;
        file.write_all(include_str!("../frontend/token.rs").as_bytes())
    }

    pub fn create_lexer(path: &Path) -> std::io::Result<()> {
        let path = path.join("lexer");
        if !path.exists() {
            create_dir(&path)?;
        }

        let mut file = File::create(path.join("mod.rs"))?;
        file.write_all(include_str!("../frontend/lexer/mod.rs").as_bytes())?;

        let path = path.join("imp.rs");
        if !path.exists() {
            let mut file = File::create(path)?;
            file.write_all(
                b"use super::*;\
                \n\
                \nimpl Lexer {\
                \n    pub fn state_start(&mut self) -> Transition {\
                \n        match self.consume() {\
                \n            // TODO\
                \n            None => self.emit(TokenKind::EOF),\
                \n            _ => {\
                \n                self.error(\"invalid token\");\
                \n                self.ignore()\
                \n            }\
                \n        }\
                \n    }\
                \n}",
            )?;
        }
        Ok(())
    }

    pub fn create_symbol(path: &Path) -> std::io::Result<()> {
        let path = path.join("symbol");
        if !path.exists() {
            create_dir(&path)?;
        }

        let mut file = File::create(path.join("mod.rs"))?;
        file.write_all(include_str!("../frontend/symbol/mod.rs").as_bytes())?;

        let path = path.join("imp.rs");
        if !path.exists() {
            let mut file = File::create(path)?;

            file.write_all(
                b"use super::*;\
                \n\
                \nmacro_rules! predefine {\
                \n    ( $([$id:ident, $name:expr]),* $(,)? ) => {\
                \n        #[repr(usize)]\
                \n        #[allow(clippy::upper_case_acronyms)]\
                \n        enum Predef {\
                \n            EMPTY,\
                \n            $($id),*\
                \n        }\
                \n        impl Symbol {\
                \n            pub const EMPTY: Symbol = Symbol(Predef::EMPTY as u32);\
                \n            $(\
                \n            pub const $id: Symbol = Symbol(Predef::$id as u32);\
                \n            )*\
                \n        }\
                \n        impl StringTable {\
                \n            pub fn init(&mut self) {\
                \n                self.alloc(\"\");\
                \n                $(self.alloc($name);)*\
                \n            }\
                \n        }\
                \n    }\
                \n}\
                \n\
                \npredefine! {\
                \n    // TODO\
                \n}",
            )?;
        }
        Ok(())
    }

    pub fn create_diag(path: &Path) -> std::io::Result<()> {
        let path = path.join("diag");
        if !path.exists() {
            create_dir(&path)?;
        }

        let mut file = File::create(path.join("mod.rs"))?;
        file.write_all(include_str!("../frontend/diag/mod.rs").as_bytes())?;

        let path = path.join("imp.rs");
        if !path.exists() {
            let mut file = File::create(path)?;
            file.write_all(
                b"use super::super::parser::*;\
                \n\
                \n#[derive(Debug)]\
                \npub enum Code {\
                \n    SyntaxError(Vec<TokenKind>),\
                \n    ParserError(&'static str),\
                \n    // TODO
                \n}\
                \n\
                \nimpl From<Vec<TokenKind>> for Code {\
                \n    fn from(error: Vec<TokenKind>) -> Code {\
                \n        Code::SyntaxError(error)\
                \n    }\
                \n}\
                \n\
                \nimpl From<&'static str> for Code {\
                \n    fn from(error: &'static str) -> Code {\
                \n        Code::ParserError(error)\
                \n    }\
                \n}\
                \n\
                \nimpl std::fmt::Display for Code {\
                \n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\
                \n        match self {\
                \n            Code::SyntaxError(expected) => {\
                \n                if expected.len() > 1 {\
                \n                    write!(f, \"invalid syntax, expected one of: \")?;\
                \n                } else {\
                \n                    write!(f, \"invalid syntax, expected: \")?;\
                \n                }\
                \n                let mut count = 0;\
                \n                for e in expected {\
                \n                    count += 1;\
                \n                    let s = format!( \"{}\", e);\
                \n                    let s = if s.starts_with('<') && s.ends_with('>') && s.len() > 2 {\
                \n                        s\
                \n                    } else {\
                \n                        format!(\"'{}'\", s)\
                \n                    };\
                \n                    if count < expected.len() {\
                \n                        write!(f, \"{}, \", s)?;\
                \n                    } else {\
                \n                        write!(f, \"{}\", s)?;\
                \n                    }\
                \n                }\
                \n                write!(f, \".\")\
                \n            }\
                \n            Code::ParserError(msg) => {\
                \n                write!(f, \"{}\", msg)\
                \n            }\
                \n        }\
                \n    }\
                \n}",
            )?;
        }
        Ok(())
    }

    pub fn create_ast(path: &Path) -> std::io::Result<()> {
        let path = path.join("ast");
        if !path.exists() {
            create_dir(&path)?;
        }

        let mut file = File::create(path.join("mod.rs"))?;
        file.write_all(include_str!("../frontend/ast/mod.rs").as_bytes())?;

        let path = path.join("imp.rs");
        if !path.exists() {
            let mut file = File::create(path)?;
            file.write_all(
                b"use super::*;\
                \n\
                \n// TODO",
            )?;
        }
        Ok(())
    }

    fn output_element(
        element: &Element,
        output: &mut File,
        common_pars: &str,
        common_args: &str,
        error_type: &str,
    ) -> std::io::Result<()> {
        if !element.attr.used.get() {
            return Ok(());
        }
        match &element.kind {
            ElementKind::Start {
                ret,
                pars,
                regex,
                action,
            } => {
                let pars = if pars.is_empty() {
                    "".to_string()
                } else {
                    format!(", {}", pars)
                };
                let ret = if ret.is_empty() {
                    "()".to_string()
                } else {
                    format!("{}", ret)
                };
                output.write_all(
                    format!(
                        "    fn start<Input: TokenStream>(depth: u16, input: &mut Input{0}{1}) -> Result<{2}, {3}> {{\n",
                        common_pars, pars, ret, error_type
                    )
                    .as_bytes(),
                )?;
                if let Some(Element {
                    kind: ElementKind::Action { code, .. },
                    ..
                }) = action.get()
                {
                    let code = code.as_string();
                    let code = if code.contains('\n') {
                        code
                    } else {
                        "    ".to_string() + code.trim()
                    };
                    output.write_all(
                        format!("    // semantic action 0\n{}\n", code)
                            .indent(1)
                            .as_bytes(),
                    )?;
                }
                Self::output_regex(regex, output, common_args, 2)?;
                output.write_all(b"    }\n")?;
            }
            ElementKind::Rule {
                name,
                ret,
                pars,
                regex,
                action,
            } => {
                let pars = if pars.is_empty() {
                    "".to_string()
                } else {
                    format!(", {}", pars)
                };
                let ret = if ret.is_empty() {
                    "()".to_string()
                } else {
                    format!("{}", ret)
                };
                output.write_all(
                    format!(
                        "    fn r#{}<Input: TokenStream>(depth: u16, input: &mut Input{}{}) -> Result<{}, {}> {{\n",
                        name, common_pars, pars, ret, error_type
                    )
                    .as_bytes()
                )?;

                output.write_all("check_limit!(input, depth);\n".indent(2).as_bytes())?;

                if let Some(Element {
                    kind: ElementKind::Action { code, .. },
                    ..
                }) = action.get()
                {
                    let code = code.as_string();
                    let code = if code.contains('\n') {
                        code
                    } else {
                        "    ".to_string() + code.trim()
                    };
                    output.write_all(
                        format!("    // semantic action 0\n{}\n", code)
                            .indent(1)
                            .as_bytes(),
                    )?;
                }
                Self::output_regex(regex, output, common_args, 2)?;
                output.write_all(b"    }\n")?;
            }
            _ => {}
        }
        Ok(())
    }

    fn get_predicate(regex: &Regex) -> String {
        match &regex.kind {
            RegexKind::Concat { ops, .. } => match &ops[0].kind {
                RegexKind::Predicate { elem, .. } => {
                    if let Some(Element {
                        kind: ElementKind::Predicate { code, .. },
                        ..
                    }) = elem.get()
                    {
                        format!(" if {}", code.to_string().trim())
                    } else {
                        "".to_string()
                    }
                }
                _ => "".to_string(),
            },
            RegexKind::Paren { op } => Self::get_predicate(op),
            _ => "".to_string(),
        }
    }

    fn output_error_handler(
        error: &Regex,
        output: &mut File,
        common_args: &str,
        level: usize,
    ) -> std::io::Result<()> {
        output.write_all(
            format!(
                "}})().or_else(|error_code| {{\
               \n    // error handling\
               \n    if input.current().kind == TokenKind::EOF {{\
               \n        return Err(error_code);\
               \n    }}\
               \n    let error_range = input.current().range;\
               \n    loop {{\
               \n        match input.current().kind {{\
               \n            {} => {{\n",
                error.follow().pattern(3),
            )
            .indent(level - 1)
            .as_bytes(),
        )?;
        Self::output_regex(error, output, common_args, level + 3)?;
        output.write_all(
            "                return Ok(())\
           \n            }\n"
                .indent(level - 1)
                .as_bytes(),
        )?;
        if !error.cancel().is_empty() {
            output.write_all(
                format!(
                    "            {} => {{\
                   \n                return Err(error_code)\
                   \n            }}\n",
                    error.cancel().pattern(3),
                )
                .indent(level - 1)
                .as_bytes(),
            )?;
        }
        output.write_all(
            "            _ => {\
           \n                input.advance();\
           \n            }\
           \n       }\
           \n   }\
           \n})?;\n"
                .indent(level - 1)
                .as_bytes(),
        )
    }

    fn output_regex(
        regex: &Regex,
        output: &mut File,
        common_args: &str,
        level: usize,
    ) -> std::io::Result<()> {
        match &regex.kind {
            RegexKind::Id { name, elem } => match elem.get().unwrap().kind {
                ElementKind::Rule { pars, .. } => {
                    let args = Self::par_to_arg(pars.as_str());
                    let args = if args.is_empty() {
                        "".to_string()
                    } else if common_args.is_empty() {
                        args
                    } else {
                        format!(", {}", args)
                    };
                    output.write_all(
                        format!(
                            "let r#{0} = Self::r#{0}(depth + 1, input{1}{2})?;\n",
                            name, common_args, args
                        )
                        .indent(level)
                        .as_bytes(),
                    )?;
                }
                ElementKind::Token { .. } => {
                    output.write_all(
                        format!("let r#{0} = consume_{0}!(input);\n", name)
                            .indent(level)
                            .as_bytes(),
                    )?;
                }
                _ => unreachable!(),
            },
            RegexKind::Str { elem, .. } => match elem.get().unwrap().kind {
                ElementKind::Token { name, .. } => {
                    output.write_all(
                        format!("let r#{0} = consume_{0}!(input);\n", name)
                            .indent(level)
                            .as_bytes(),
                    )?;
                }
                _ => unreachable!(),
            },
            RegexKind::Concat { ops, error } => {
                let level = if error.get().is_some() {
                    output.write_all("(|| {\n".indent(level).as_bytes())?;
                    level + 1
                } else {
                    level
                };
                for op in ops {
                    if let RegexKind::ErrorHandler { .. } = op.kind {
                        output.write_all(
                            format!(
                                "match input.current().kind {{\
                               \n    {} => {{\
                               \n        Ok(())\
                               \n    }}\
                               \n    _ => {{\
                               \n        return err![{}]\
                               \n    }}\
                               \n}}\n",
                                op.follow().pattern(1),
                                op.follow().error(5)
                            )
                            .indent(level)
                            .as_bytes(),
                        )?;
                    } else {
                        Self::output_regex(op, output, common_args, level)?;
                    }
                }
                if let Some(error) = error.get() {
                    Self::output_error_handler(error, output, common_args, level)?;
                }
            }
            RegexKind::Or { ops, error } => {
                let level = if error.get().is_some() {
                    output.write_all("(|| {\n".indent(level).as_bytes())?;
                    level + 1
                } else {
                    level
                };
                output.write_all("match input.current().kind {\n".indent(level).as_bytes())?;
                for op in ops {
                    // check if this is the error rule, if so ignore it here
                    if let RegexKind::ErrorHandler { .. } = &op.kind {
                        continue;
                    }
                    output.write_all(
                        format!(
                            "{}{} => {{\n",
                            op.predict().pattern(0),
                            Self::get_predicate(op)
                        )
                        .indent(level + 1)
                        .as_bytes(),
                    )?;
                    Self::output_regex(op, output, common_args, level + 2)?;
                    if error.get().is_some() {
                        output.write_all("Ok(())\n".indent(level + 2).as_bytes())?;
                    }
                    output.write_all("}\n".indent(level + 1).as_bytes())?;
                }
                output.write_all(
                    format!(
                        "    _ => {{\
                       \n        return err![{}]\
                       \n    }}\
                       \n}}\n",
                        regex.predict().error(5)
                    )
                    .indent(level)
                    .as_bytes(),
                )?;
                if let Some(error) = error.get() {
                    Self::output_error_handler(error, output, common_args, level)?;
                }
            }
            RegexKind::Star { op } => {
                output.write_all(
                    format!(
                        "loop {{\
                       \n    match input.current().kind {{\
                       \n        {}{} => {{\n",
                        op.first().pattern(2),
                        Self::get_predicate(op)
                    )
                    .indent(level)
                    .as_bytes(),
                )?;
                Self::output_regex(op, output, common_args, level + 3)?;
                output.write_all(
                    format!(
                        "        }}\
                       \n        {} => break,\
                       \n        _ => {{\
                       \n            return err![{}]\
                       \n        }}\
                       \n    }}\
                       \n}}\n",
                        regex.follow().pattern(2),
                        regex.predict().error(6)
                    )
                    .indent(level)
                    .as_bytes(),
                )?;
            }
            RegexKind::Plus { op } => {
                output.write_all(
                    format!(
                        "let mut is_first = true;\
                       \nloop {{\
                       \n    match input.current().kind {{\
                       \n        {}{} => {{\n",
                        op.first().pattern(2),
                        Self::get_predicate(op),
                    )
                    .indent(level)
                    .as_bytes(),
                )?;
                Self::output_regex(op, output, common_args, level + 3)?;
                output.write_all(
                    format!(
                        "        }}\
                       \n        {0} if !is_first => break,\
                       \n        _ if is_first => {{\
                       \n            return err![{1}]\
                       \n        }}\
                       \n        _ => {{\
                       \n            return err![{1},\
                       \n                        {2}]\
                       \n        }}\
                       \n    }}\
                       \n    is_first = false;\
                       \n}}\n",
                        regex.follow().pattern(2),
                        regex.first().error(6),
                        regex.follow().error(6)
                    )
                    .indent(level)
                    .as_bytes(),
                )?;
            }
            RegexKind::Option { op } => {
                let name = match &op.kind {
                    RegexKind::Id { name, .. } => *name,
                    RegexKind::Str { elem, .. } => match elem.get() {
                        Some(Element {
                            kind: ElementKind::Token { name, .. },
                            ..
                        }) => *name,
                        _ => Symbol::EMPTY,
                    },
                    _ => Symbol::EMPTY,
                };
                output.write_all(
                    format!(
                        "{}match input.current().kind {{\
                       \n    {}{} => {{\n",
                        if !name.is_empty() {
                            format!("let r#{} = ", name)
                        } else {
                            "".to_string()
                        },
                        op.first().pattern(1),
                        Self::get_predicate(op)
                    )
                    .indent(level)
                    .as_bytes(),
                )?;
                Self::output_regex(op, output, common_args, level + 2)?;
                if !name.is_empty() {
                    output.write_all(format!("Some({})\n", name).indent(level + 2).as_bytes())?;
                }
                output.write_all(
                    format!(
                        "    }}\
                       \n    {} => {}\
                       \n    _ => {{\
                       \n        return err![{}]\
                       \n    }}\
                       \n}}{}\n",
                        regex.follow().pattern(1),
                        if !name.is_empty() { "None," } else { "{}" },
                        regex.predict().error(5),
                        if !name.is_empty() { ";" } else { "" },
                    )
                    .indent(level)
                    .as_bytes(),
                )?;
            }
            RegexKind::Paren { op } => {
                Self::output_regex(op, output, common_args, level)?;
            }
            RegexKind::Action { val, elem } => {
                let code = match elem.get() {
                    Some(Element {
                        kind: ElementKind::Action { code, .. },
                        ..
                    }) => {
                        let code = code.as_string();
                        if code.contains('\n') {
                            code
                        } else {
                            "    ".to_string() + code.trim()
                        }
                    }
                    _ => format!(
                        "    todo!(\"semantic action {} at {}\");\n",
                        val,
                        regex.range().start
                    ),
                };
                output.write_all(
                    format!("    // semantic action {}\n{}\n", val, code)
                        .indent(level - 1)
                        .as_bytes(),
                )?;
            }
            RegexKind::ErrorHandler { val, elem } => {
                let code = match elem.get() {
                    Some(Element {
                        kind: ElementKind::ErrorHandler { code, .. },
                        ..
                    }) => {
                        let code = code.as_string();
                        if code.contains('\n') {
                            code
                        } else {
                            "    ".to_string() + code.trim()
                        }
                    }
                    _ => format!(
                        "    todo!(\"error handler {} at {}\");\n",
                        val,
                        regex.range().start
                    ),
                };
                output.write_all(
                    format!("    // error handler {}\n{}\n", val, code)
                        .indent(level - 1)
                        .as_bytes(),
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Outputs the code of the preamble section.
    fn output_preamble(module: &Module, output: &mut File) -> std::io::Result<()> {
        if let Some(preamble) = module.preamble.get() {
            output.write_all(b"// preamble\n")?;
            if let ElementKind::Preamble { code } = preamble.kind {
                for l in code.to_string().trim().lines() {
                    output.write_all(l.trim().as_bytes())?;
                    output.write_all(b"\n")?;
                }
            } else {
                unreachable!()
            }
        }
        output.write_all(b"\n")
    }

    /// Outputs the TokenKind enumeration.
    fn output_tokens(module: &Module, output: &mut File) -> std::io::Result<()> {
        output.write_all(
            b"#[derive(PartialEq, Clone, Debug)]\n\
              pub enum TokenKind {\
            \n    EOF,\n",
        )?;
        for element in module.elements.iter() {
            if let ElementKind::Token { name, ty, .. } = element.kind {
                if ty.is_empty() {
                    output.write_all(format!("    {},\n", name).as_bytes())?;
                } else {
                    output.write_all(format!("    {}({}),\n", name, ty).as_bytes())?;
                }
            }
        }
        output.write_all(b"}\n\n")
    }

    /// Outputs the pattern_* macros.
    fn output_patterns(module: &Module, output: &mut File) -> std::io::Result<()> {
        output.write_all(b"macro_rules! pattern_EOF { () => { TokenKind::EOF } }\n")?;
        for element in module.elements.iter() {
            if let ElementKind::Token { name, ty, .. } = element.kind {
                let s = if ty.is_empty() {
                    format!(
                        "macro_rules! pattern_{0} {{ () => {{ TokenKind::{0} }} }}\n",
                        name
                    )
                } else {
                    format!(
                        "macro_rules! pattern_{0} {{ () => {{ TokenKind::{0}(_) }} }}\n",
                        name
                    )
                };
                output.write_all(s.as_bytes())?;
            }
        }
        output.write_all(b"\n")
    }

    /// Outputs the default_* macros.
    fn output_defaults(module: &Module, output: &mut File) -> std::io::Result<()> {
        output.write_all(b"macro_rules! default_EOF { () => { TokenKind::EOF } }\n")?;
        for element in module.elements.iter() {
            if let ElementKind::Token { name, ty, .. } = element.kind {
                let name = name.to_string();
                if name.starts_with('_') {
                    continue;
                }
                let s = if ty.is_empty() {
                    format!(
                        "macro_rules! default_{0} {{ () => {{ TokenKind::{0} }} }}\n",
                        name
                    )
                } else {
                    format!(
                        "macro_rules! default_{0} {{ () => {{ TokenKind::{0}({1}::default()) }} }}\n",
                        name, ty
                    )
                };
                output.write_all(s.as_bytes())?;
            }
        }
        output.write_all(b"\n")
    }

    fn output_error(module: &Module, output: &mut File) -> std::io::Result<()> {
        if let Some(Element {
            kind: ElementKind::ErrorCode { code },
            ..
        }) = module.error.get()
        {
            output.write_all(format!("macro_rules! err {{ [$($tk:expr),*] => {{ Err({}::from(vec![$($tk),*])) }} }}\n\n", code.as_str().trim()).as_bytes())
        } else {
            output.write_all(b"macro_rules! err { [$($tk:expr),*] => { Err(vec![$($tk),*]) } }\n\n")
        }
    }

    fn output_check_limit(module: &Module, output: &mut File) -> std::io::Result<()> {
        let limit = if let Some(Element {
            kind: ElementKind::Limit { depth },
            ..
        }) = module.limit.get()
        {
            *depth
        } else {
            128
        };
        if let Some(Element {
            kind: ElementKind::ErrorCode { code },
            ..
        }) = module.error.get()
        {
            output.write_all(
                format!(
                    "#[allow(unused_macros)]\
                   \nmacro_rules! check_limit {{\
                   \n    ($input:ident, $depth:expr) => {{\
                   \n        if $depth > {} {{\
                   \n            $input.finalize();\
                   \n            return Err({}::from(\"exceeded recursion depth limit\"));\
                   \n        }}\
                   \n    }}\
                   \n}}\n\n",
                    limit,
                    code.as_str().trim()
                )
                .as_bytes(),
            )
        } else {
            output.write_all(
                format!(
                    "#[allow(unused_macros)]\
                   \nmacro_rules! check_limit {{\
                   \n    ($input:ident, $depth:expr) => {{\
                   \n        if $depth > {} {{\
                   \n            panic!(\"exceeded recursion depth limit\");\
                   \n        }}\
                   \n    }}\
                   \n}}\n\n",
                    limit
                )
                .as_bytes(),
            )
        }
    }

    /// Outputs the consume_* macros.
    fn output_consumes(module: &Module, output: &mut File) -> std::io::Result<()> {
        for element in module.elements.iter() {
            if let ElementKind::Token { name, ty, .. } = element.kind {
                let name = name.to_string();
                if name.starts_with('_') {
                    continue;
                }
                let s = if ty.is_empty() {
                    format!(
                        "macro_rules! consume_{0} {{\
                       \n    ($input:ident) => {{\
                       \n        if let TokenKind::{0} = $input.current().kind {{\
                       \n            let range = $input.current().range;\
                       \n            $input.advance();\
                       \n            range\
                       \n        }} else {{\
                       \n            return err![default_{0}!()]\
                       \n        }}\
                       \n    }}\
                       \n}}\n",
                        name
                    )
                } else {
                    format!(
                        "macro_rules! consume_{0} {{\
                       \n    ($input:ident) => {{\
                       \n        if let TokenKind::{0}(value) = $input.current().kind {{\
                       \n            let range = $input.current().range;\
                       \n            $input.advance();\
                       \n            (value, range)\
                       \n        }} else {{\
                       \n            return err![default_{0}!()]\
                       \n        }}\
                       \n    }}\
                       \n}}\n",
                        name
                    )
                };
                output.write_all(s.as_bytes())?;
            }
        }
        output.write_all(b"\n")
    }

    /// Outputs the fmt::Display trait impl for TokenKind.
    fn output_display(module: &Module, output: &mut File) -> std::io::Result<()> {
        output.write_all(
            b"use std::fmt;\n\
              impl fmt::Display for TokenKind {\
            \n    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {\
            \n        match self {\
            \n            pattern_EOF!() => write!(f, \"end of file\"),\n",
        )?;
        for element in module.elements.iter() {
            if let ElementKind::Token { name, sym, .. } = element.kind {
                let s = if sym.is_empty() {
                    format!(
                        "            pattern_{0}!() => write!(f, \"{{}}\", r###\"{0}\"###),\n",
                        name
                    )
                } else {
                    format!(
                        "            pattern_{}!() => write!(f, \"{{}}\", r###\"{}\"###),\n",
                        name, sym
                    )
                };
                output.write_all(s.as_bytes())?;
            }
        }
        output.write_all(
            b"        }\
            \n    }\
            \n}\n\n",
        )
    }

    /// Outputs the Parser struct and impl.
    fn output_parser(module: &Module, output: &mut File) -> std::io::Result<()> {
        let common_pars = match module.parameters.get() {
            Some(Element {
                kind: ElementKind::Parameters { code },
                ..
            }) => {
                format!(", {}", code.to_string().trim())
            }
            _ => "".to_string(),
        };
        let common_args = match module.parameters.get() {
            Some(Element {
                kind: ElementKind::Parameters { code },
                ..
            }) => {
                format!(", {}", Self::par_to_arg(code.to_string().trim()))
            }
            _ => "".to_string(),
        };
        let error_type = match module.error.get() {
            Some(Element {
                kind: ElementKind::ErrorCode { code },
                ..
            }) => code.as_str().trim(),
            _ => "Vec<TokenKind>",
        };
        let mut start_ret = "()".to_string();
        let mut start_pars = "".to_string();
        for element in module.elements.iter() {
            if let ElementKind::Start { ret, pars, .. } = element.kind {
                if !ret.is_empty() {
                    start_ret = format!("{}", ret)
                }
                if !pars.is_empty() {
                    start_pars = format!(", {}", pars)
                }
            }
        }
        output.write_all(
            format!(
                "pub struct Parser;\
               \n\
               \nimpl<'a> Parser {{\
               \n    pub fn parse<Input: TokenStream>(input: &mut Input{0}{1}) -> Result<{2}, {4}> {{\
               \n        input.advance();\
               \n        let out = Self::start(0, input{3})?;\
               \n        if input.current().kind != TokenKind::EOF {{\
               \n            return err![default_EOF!()]\
               \n        }}\
               \n        Ok(out)\
               \n    }}\n",
                common_pars, start_pars, start_ret, common_args, error_type
            )
            .as_bytes(),
        )?;
        for element in module.elements.iter() {
            Self::output_element(element, output, &common_pars, &common_args, error_type)?;
        }
        output.write_all(b"}\n")
    }

    fn par_to_arg(pars: &str) -> String {
        pars.rsplit(':')
            .skip(1)
            .collect::<Vec<&str>>()
            .iter()
            .rev()
            .map(|s| s.rsplit(',').next().unwrap())
            .collect::<Vec<&str>>()
            .join(",")
    }
}
