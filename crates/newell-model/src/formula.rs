// Newell - Parametric CAD Software
// Copyright (c) 2026 Martin Harambat
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//! Bounded, dimension-checked formula parsing. Source and reference identities are independent.
use std::collections::BTreeMap;

use crate::{Angle, Length, Quantity, QuantityKind, Scalar, Unit};

/// Stable registry identity; names never act as identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterId(pub(crate) u64);
impl ParameterId {
    /// Returns the document-local identity.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}
/// UTF-8 byte boundaries in the original source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}
/// Stable identity bound to one reference token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundReference {
    pub parameter: ParameterId,
    pub span: SourceSpan,
}
/// Machine-readable formula diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaErrorCode {
    Syntax,
    UnknownName,
    MissingParameter,
    Cycle,
    Dimension,
    DivideByZero,
    FunctionDomain,
    NonFinite,
    Underflow,
    Range,
    TooComplex,
    ForwardDependency,
}
/// Formula failure pointing at the original editable source.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code:?} at {span:?}")]
pub struct FormulaError {
    pub code: FormulaErrorCode,
    pub span: SourceSpan,
}
fn error(code: FormulaErrorCode, span: SourceSpan) -> FormulaError {
    FormulaError { code, span }
}
#[derive(Debug, Clone, PartialEq)]
enum Node {
    Literal(Quantity),
    Reference(ParameterId),
    Unary(bool, usize),
    Binary(u8, usize, usize),
    Call(Function, usize, Option<usize>),
}
#[derive(Debug, Clone, Copy, PartialEq)]
enum Function {
    Sin,
    Cos,
    Tan,
    Abs,
    Min,
    Max,
}
/// Parsed source with stable token bindings and an immutable bounded AST.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedExpression {
    source: String,
    name: Option<String>,
    nodes: Vec<(Node, SourceSpan)>,
    references: Vec<BoundReference>,
    root: usize,
    destination: QuantityKind,
}
impl ParsedExpression {
    /// Required destination dimension.
    #[must_use]
    pub const fn destination(&self) -> QuantityKind {
        self.destination
    }
    /// Exact source, including spaces and optional header.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Optional top-level definition header.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    /// References resolved when this expression was accepted.
    #[must_use]
    pub fn references(&self) -> &[BoundReference] {
        &self.references
    }
    /// Evaluates an already-bound AST without looking up names again.
    ///
    /// # Errors
    /// Reports missing identities, invalid arithmetic, dimensions, or final range.
    pub fn evaluate(
        &self,
        mut resolve: impl FnMut(ParameterId) -> Option<Quantity>,
    ) -> Result<Quantity, FormulaError> {
        let mut values: Vec<Quantity> = Vec::with_capacity(self.nodes.len());
        for (node, span) in &self.nodes {
            let get = |index: usize| {
                values
                    .get(index)
                    .copied()
                    .ok_or_else(|| error(FormulaErrorCode::Syntax, *span))
            };
            let result = match *node {
                Node::Literal(value) => value,
                Node::Reference(id) => {
                    resolve(id).ok_or_else(|| error(FormulaErrorCode::MissingParameter, *span))?
                }
                Node::Unary(negative, index) => {
                    let q = get(index)?;
                    quantity(q.kind(), if negative { -raw(q) } else { raw(q) }, *span)?
                }
                Node::Binary(op, left, right) => arithmetic(op, get(left)?, get(right)?, *span)?,
                Node::Call(function, first, second) => {
                    function.apply(get(first)?, second.map(get).transpose()?, *span)?
                }
            };
            values.push(result);
        }
        let span = self.nodes[self.root].1;
        let result = values[self.root];
        if result.kind() != self.destination {
            return Err(error(FormulaErrorCode::Dimension, span));
        }
        let maximum = if result.kind() == QuantityKind::Angle {
            9_999_999.9_f64.to_radians()
        } else {
            99_999_999.9
        };
        if raw(result) < 0. || raw(result) > maximum {
            return Err(error(FormulaErrorCode::Range, span));
        }
        Ok(result)
    }
}
fn raw(q: Quantity) -> f64 {
    match q {
        Quantity::Length(v) => v.as_millimetres(),
        Quantity::Angle(v) => v.as_radians(),
        Quantity::Scalar(v) => v.value(),
    }
}
fn quantity(kind: QuantityKind, value: f64, span: SourceSpan) -> Result<Quantity, FormulaError> {
    let q = match kind {
        QuantityKind::Length => Length::millimetres(value).map(Quantity::Length),
        QuantityKind::Angle => Angle::radians(value).map(Quantity::Angle),
        QuantityKind::Scalar => Scalar::new(value).map(Quantity::Scalar),
    };
    q.map_err(|_| error(FormulaErrorCode::NonFinite, span))
}
fn arithmetic(
    op: u8,
    a: Quantity,
    b: Quantity,
    span: SourceSpan,
) -> Result<Quantity, FormulaError> {
    use FormulaErrorCode::{Dimension, DivideByZero, Underflow};
    let (x, y) = (raw(a), raw(b));
    let (kind, value) = match op {
        b'+' | b'-' if a.kind() == b.kind() => (a.kind(), if op == b'+' { x + y } else { x - y }),
        b'*' if a.kind() == QuantityKind::Scalar => (b.kind(), x * y),
        b'*' if b.kind() == QuantityKind::Scalar => (a.kind(), x * y),
        b'/' => {
            if y == 0. {
                return Err(error(DivideByZero, span));
            }
            if b.kind() == QuantityKind::Scalar {
                (a.kind(), x / y)
            } else if a.kind() == b.kind() {
                (QuantityKind::Scalar, x / y)
            } else {
                return Err(error(Dimension, span));
            }
        }
        _ => return Err(error(Dimension, span)),
    };
    if matches!(op, b'*' | b'/') && value == 0. && x != 0. && y != 0. {
        return Err(error(Underflow, span));
    }
    quantity(kind, value, span)
}
impl Function {
    fn apply(
        self,
        a: Quantity,
        b: Option<Quantity>,
        span: SourceSpan,
    ) -> Result<Quantity, FormulaError> {
        use FormulaErrorCode::{Dimension, FunctionDomain, Syntax};
        match self {
            Self::Abs => quantity(a.kind(), raw(a).abs(), span),
            Self::Min | Self::Max => {
                let b = b.ok_or_else(|| error(Syntax, span))?;
                if a.kind() != b.kind() {
                    return Err(error(Dimension, span));
                }
                quantity(
                    a.kind(),
                    if self == Self::Min {
                        raw(a).min(raw(b))
                    } else {
                        raw(a).max(raw(b))
                    },
                    span,
                )
            }
            Self::Sin | Self::Cos | Self::Tan => {
                if a.kind() != QuantityKind::Angle {
                    return Err(error(Dimension, span));
                }
                let x = raw(a);
                if self == Self::Tan && x.cos().abs() <= 1e-12 {
                    return Err(error(FunctionDomain, span));
                }
                let value = match self {
                    Self::Sin => x.sin(),
                    Self::Cos => x.cos(),
                    _ => x.tan(),
                };
                quantity(QuantityKind::Scalar, value, span)
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Number(Quantity),
    Ref(String),
    Ident(String),
    Symbol(u8),
}
#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    span: SourceSpan,
}
fn lex(source: &str, start: usize) -> Result<Vec<Token>, FormulaError> {
    use FormulaErrorCode::{Syntax, TooComplex};
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut pos = start;
    while pos < bytes.len() {
        if bytes[pos].is_ascii_whitespace() {
            pos += 1;
            continue;
        }
        let begin = pos;
        let kind = match bytes[pos] {
            b'0'..=b'9' | b'.' => {
                while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
                    pos += 1;
                }
                if bytes.get(pos) == Some(&b'.') {
                    pos += 1;
                    while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
                        pos += 1;
                    }
                }
                if bytes.get(pos).is_some_and(|b| matches!(b, b'e' | b'E')) {
                    pos += 1;
                    if bytes.get(pos).is_some_and(|b| matches!(b, b'+' | b'-')) {
                        pos += 1;
                    }
                    while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
                        pos += 1;
                    }
                }
                let number_end = pos;
                while bytes.get(pos).is_some_and(u8::is_ascii_whitespace) {
                    pos += 1;
                }
                let suffix_start = pos;
                while bytes.get(pos).is_some_and(u8::is_ascii_alphabetic) {
                    pos += 1;
                }
                let unit = match &source[suffix_start..pos] {
                    "mm" => Unit::Millimetre,
                    "deg" => Unit::Degree,
                    "" => Unit::Unitless,
                    _ => {
                        return Err(error(
                            Syntax,
                            SourceSpan {
                                start: begin,
                                end: pos,
                            },
                        ));
                    }
                };
                if pos == suffix_start {
                    pos = number_end;
                }
                let span = SourceSpan {
                    start: begin,
                    end: pos,
                };
                let value = Quantity::parse_literal(&source[begin..pos], unit).map_err(|e| {
                    error(
                        match e {
                            crate::ModelError::QuantityUnderflow => FormulaErrorCode::Underflow,
                            crate::ModelError::QuantityOverflow
                            | crate::ModelError::NonFiniteQuantity => FormulaErrorCode::NonFinite,
                            _ => Syntax,
                        },
                        span,
                    )
                })?;
                TokenKind::Number(value)
            }
            b'#' => {
                pos += 1;
                let name_start = pos;
                scan_name(bytes, &mut pos);
                if !valid_name(&source[name_start..pos]) {
                    return Err(error(
                        Syntax,
                        SourceSpan {
                            start: begin,
                            end: pos,
                        },
                    ));
                }
                TokenKind::Ref(source[name_start..pos].to_owned())
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                scan_name(bytes, &mut pos);
                TokenKind::Ident(source[begin..pos].to_owned())
            }
            symbol @ (b'+' | b'-' | b'*' | b'/' | b'(' | b')' | b',') => {
                pos += 1;
                TokenKind::Symbol(symbol)
            }
            _ => {
                return Err(error(
                    Syntax,
                    SourceSpan {
                        start: begin,
                        end: begin + source[begin..].chars().next().map_or(0, char::len_utf8),
                    },
                ));
            }
        };
        if tokens.len() >= 128 {
            return Err(error(
                TooComplex,
                SourceSpan {
                    start: begin,
                    end: pos,
                },
            ));
        }
        tokens.push(Token {
            kind,
            span: SourceSpan {
                start: begin,
                end: pos,
            },
        });
    }
    Ok(tokens)
}
fn scan_name(bytes: &[u8], pos: &mut usize) {
    while bytes
        .get(*pos)
        .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
    {
        *pos += 1;
    }
}
pub(crate) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 48
        && name
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
struct Parser<'a> {
    retained: BTreeMap<usize, BoundReference>,
    tokens: Vec<Token>,
    pos: usize,
    nodes: Vec<(Node, SourceSpan)>,
    refs: Vec<BoundReference>,
    names: &'a BTreeMap<String, ParameterId>,
    end: usize,
}
impl Parser<'_> {
    fn add(&mut self, node: Node, span: SourceSpan) -> Result<usize, FormulaError> {
        if self.nodes.len() >= 64 {
            return Err(error(FormulaErrorCode::TooComplex, span));
        }
        let index = self.nodes.len();
        self.nodes.push((node, span));
        Ok(index)
    }
    fn symbol(&mut self, symbol: u8) -> bool {
        if self
            .tokens
            .get(self.pos)
            .is_some_and(|t| t.kind == TokenKind::Symbol(symbol))
        {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn required(&mut self, symbol: u8) -> Result<(), FormulaError> {
        if self.symbol(symbol) {
            Ok(())
        } else {
            Err(error(FormulaErrorCode::Syntax, self.here()))
        }
    }
    fn here(&self) -> SourceSpan {
        self.tokens.get(self.pos).map_or(
            SourceSpan {
                start: self.end,
                end: self.end,
            },
            |t| t.span,
        )
    }
    fn expression(&mut self, min: u8, depth: usize) -> Result<usize, FormulaError> {
        if depth > 16 {
            return Err(error(FormulaErrorCode::TooComplex, self.here()));
        }
        let token = self
            .tokens
            .get(self.pos)
            .cloned()
            .ok_or_else(|| error(FormulaErrorCode::Syntax, self.here()))?;
        self.pos += 1;
        let mut left = match token.kind {
            TokenKind::Number(q) => self.add(Node::Literal(q), token.span)?,
            TokenKind::Ref(name) => {
                let id = self
                    .retained
                    .get(&token.span.start)
                    .filter(|reference| reference.span == token.span)
                    .map(|reference| reference.parameter)
                    .or_else(|| self.names.get(&name).copied())
                    .ok_or_else(|| error(FormulaErrorCode::UnknownName, token.span))?;
                if self.refs.len() >= 64 {
                    return Err(error(FormulaErrorCode::TooComplex, token.span));
                }
                self.refs.push(BoundReference {
                    parameter: id,
                    span: token.span,
                });
                self.add(Node::Reference(id), token.span)?
            }
            TokenKind::Symbol(op @ (b'+' | b'-')) => {
                let value = self.expression(5, depth + 1)?;
                self.add(
                    Node::Unary(op == b'-', value),
                    SourceSpan {
                        start: token.span.start,
                        end: self.nodes[value].1.end,
                    },
                )?
            }
            TokenKind::Symbol(b'(') => {
                let value = self.expression(0, depth + 1)?;
                self.required(b')')?;
                value
            }
            TokenKind::Ident(name) => {
                let function = match name.as_str() {
                    "sin" => Function::Sin,
                    "cos" => Function::Cos,
                    "tan" => Function::Tan,
                    "abs" => Function::Abs,
                    "min" => Function::Min,
                    "max" => Function::Max,
                    _ => return Err(error(FormulaErrorCode::Syntax, token.span)),
                };
                self.required(b'(')?;
                let first = self.expression(0, depth + 1)?;
                let second = if matches!(function, Function::Min | Function::Max) {
                    self.required(b',')?;
                    Some(self.expression(0, depth + 1)?)
                } else {
                    None
                };
                self.required(b')')?;
                self.add(
                    Node::Call(function, first, second),
                    SourceSpan {
                        start: token.span.start,
                        end: self.tokens[self.pos - 1].span.end,
                    },
                )?
            }
            _ => return Err(error(FormulaErrorCode::Syntax, token.span)),
        };
        while let Some(Token {
            kind: TokenKind::Symbol(op @ (b'+' | b'-' | b'*' | b'/')),
            ..
        }) = self.tokens.get(self.pos)
        {
            let op = *op;
            let precedence = if matches!(op, b'+' | b'-') { 1 } else { 3 };
            if precedence < min {
                break;
            }
            self.pos += 1;
            let right = self.expression(precedence + 1, depth + 1)?;
            left = self.add(
                Node::Binary(op, left, right),
                SourceSpan {
                    start: self.nodes[left].1.start,
                    end: self.nodes[right].1.end,
                },
            )?;
        }
        Ok(left)
    }
}
/// Parses the approved formula grammar and binds reference tokens once.
///
/// # Errors
/// Reports syntax, complexity and unknown names without mutating the registry.
pub fn parse_expression(
    source: &str,
    destination: QuantityKind,
    names: &BTreeMap<String, ParameterId>,
) -> Result<ParsedExpression, FormulaError> {
    parse_with_retained(source, destination, names, BTreeMap::new())
}
fn parse_with_retained(
    source: &str,
    destination: QuantityKind,
    names: &BTreeMap<String, ParameterId>,
    retained: BTreeMap<usize, BoundReference>,
) -> Result<ParsedExpression, FormulaError> {
    let whole = SourceSpan {
        start: 0,
        end: source.len(),
    };
    if source.chars().count() > 256 {
        return Err(error(FormulaErrorCode::TooComplex, whole));
    }
    let (name, start) = definition_header(source)?;
    let tokens = lex(source, start)?;
    let mut parser = Parser {
        retained,
        tokens,
        pos: 0,
        nodes: Vec::new(),
        refs: Vec::new(),
        names,
        end: source.len(),
    };
    let root = parser.expression(0, 0)?;
    if parser.pos != parser.tokens.len() {
        return Err(error(FormulaErrorCode::Syntax, parser.here()));
    }
    // Only a complete signed literal inherits the destination's presentation unit.
    let unit = match destination {
        QuantityKind::Length => Unit::Millimetre,
        QuantityKind::Angle => Unit::Degree,
        QuantityKind::Scalar => Unit::Unitless,
    };
    if let Ok(literal) = Quantity::parse_literal(&source[start..], unit) {
        parser.nodes.clear();
        parser.nodes.push((
            Node::Literal(literal),
            SourceSpan {
                start,
                end: source.len(),
            },
        ));
        return Ok(ParsedExpression {
            source: source.to_owned(),
            name,
            nodes: parser.nodes,
            references: parser.refs,
            root: 0,
            destination,
        });
    }
    Ok(ParsedExpression {
        source: source.to_owned(),
        name,
        nodes: parser.nodes,
        references: parser.refs,
        root,
        destination,
    })
}

pub(crate) fn literal_expression(value: Quantity) -> ParsedExpression {
    let source = match value {
        Quantity::Length(v) => format!("{}mm", v.as_millimetres()),
        Quantity::Angle(v) => format!("{}deg", v.as_radians().to_degrees()),
        Quantity::Scalar(v) => v.value().to_string(),
    };
    let span = SourceSpan {
        start: 0,
        end: source.len(),
    };
    ParsedExpression {
        source,
        name: None,
        nodes: vec![(Node::Literal(value), span)],
        references: Vec::new(),
        root: 0,
        destination: value.kind(),
    }
}

/// One actual editor replacement, expressed in the previous source's UTF-8 byte coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaSourceEdit {
    pub replaced: SourceSpan,
    pub inserted: String,
}
/// Rebinds edited tokens while retaining identities for tokens outside the actual edit.
///
/// # Errors
/// Rejects invalid edit coordinates, malformed formulas and newly unknown names.
pub fn edit_expression(
    previous: &ParsedExpression,
    edit: &FormulaSourceEdit,
    names: &BTreeMap<String, ParameterId>,
) -> Result<ParsedExpression, FormulaError> {
    let range = edit.replaced;
    if range.start > range.end
        || !previous.source.is_char_boundary(range.start)
        || !previous.source.is_char_boundary(range.end)
    {
        return Err(error(FormulaErrorCode::Syntax, range));
    }
    let mut source = previous.source.clone();
    source.replace_range(range.start..range.end, &edit.inserted);
    let mut retained = BTreeMap::new();
    for reference in previous.references() {
        let span = reference.span;
        // Insertion at a token boundary might extend that token; lexer span equality below
        // ensures it will be rebound, while a disjoint token keeps its identity.
        let shifted = if span.end <= range.start {
            Some(span)
        } else if span.start >= range.end {
            let start = span
                .start
                .checked_sub(range.end)
                .and_then(|v| v.checked_add(range.start))
                .and_then(|v| v.checked_add(edit.inserted.len()));
            let end = span
                .end
                .checked_sub(range.end)
                .and_then(|v| v.checked_add(range.start))
                .and_then(|v| v.checked_add(edit.inserted.len()));
            start.zip(end).map(|(start, end)| SourceSpan { start, end })
        } else {
            None
        };
        if let Some(span) = shifted {
            retained.insert(
                span.start,
                BoundReference {
                    span,
                    parameter: reference.parameter,
                },
            );
        }
    }
    parse_with_retained(&source, previous.destination, names, retained)
}

/// Reparses a bounded character-origin map emitted by the source editor.
///
/// # Errors
/// Rejects invalid provenance and newly unknown names. Untouched reference tokens retain ids.
pub fn reparse_with_origins(
    previous: &ParsedExpression,
    source: &str,
    origins: &[Option<usize>],
    names: &BTreeMap<String, ParameterId>,
) -> Result<ParsedExpression, FormulaError> {
    if source.chars().count() != origins.len() || source.chars().count() > 256 {
        return Err(error(
            FormulaErrorCode::TooComplex,
            SourceSpan {
                start: 0,
                end: source.len(),
            },
        ));
    }
    let positions: BTreeMap<_, _> = source
        .char_indices()
        .zip(origins)
        .filter_map(|((new, c), old)| old.map(|old| (old, (new, c))))
        .collect();
    let mut last_origin = None;
    for ((_, c), old) in source.char_indices().zip(origins) {
        if let Some(old) = old {
            if last_origin.is_some_and(|last| *old <= last) {
                return Err(error(
                    FormulaErrorCode::Syntax,
                    SourceSpan {
                        start: 0,
                        end: source.len(),
                    },
                ));
            }
            last_origin = Some(*old);
            if previous.source.get(*old..).and_then(|s| s.chars().next()) != Some(c) {
                return Err(error(
                    FormulaErrorCode::Syntax,
                    SourceSpan {
                        start: 0,
                        end: source.len(),
                    },
                ));
            }
        }
    }
    let mut retained = BTreeMap::new();
    for reference in previous.references() {
        if let Some(&(start, _)) = positions.get(&reference.span.start) {
            let length = reference.span.end - reference.span.start;
            if (0..length).all(|offset| {
                positions
                    .get(&(reference.span.start + offset))
                    .is_some_and(|(new, _)| *new == start + offset)
            }) {
                let span = SourceSpan {
                    start,
                    end: start + length,
                };
                retained.insert(
                    start,
                    BoundReference {
                        span,
                        parameter: reference.parameter,
                    },
                );
            }
        }
    }
    parse_with_retained(source, previous.destination, names, retained)
}

pub(crate) fn definition_header(source: &str) -> Result<(Option<String>, usize), FormulaError> {
    let whole = SourceSpan {
        start: 0,
        end: source.len(),
    };
    let mut start = source.len()
        - source
            .trim_start_matches(|c: char| c.is_ascii_whitespace())
            .len();
    if source[start..].starts_with("Fx") {
        start += 2;
        while source
            .as_bytes()
            .get(start)
            .is_some_and(u8::is_ascii_whitespace)
        {
            start += 1;
        }
        if source.as_bytes().get(start) != Some(&b'=') {
            return Err(error(FormulaErrorCode::Syntax, whole));
        }
        start += 1;
    }
    while source
        .as_bytes()
        .get(start)
        .is_some_and(u8::is_ascii_whitespace)
    {
        start += 1;
    }
    let mut name = None;
    if source.as_bytes().get(start) == Some(&b'#') {
        let begin = start + 1;
        let mut end = begin;
        scan_name(source.as_bytes(), &mut end);
        let mut after = end;
        while source
            .as_bytes()
            .get(after)
            .is_some_and(u8::is_ascii_whitespace)
        {
            after += 1;
        }
        if source[after..].starts_with(":=") {
            if !valid_name(&source[begin..end]) {
                return Err(error(
                    FormulaErrorCode::Syntax,
                    SourceSpan { start: begin, end },
                ));
            }
            name = Some(source[begin..end].to_owned());
            start = after + 2;
        }
    }
    Ok((name, start))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn evaluate(source: &str, kind: QuantityKind) -> Result<Quantity, FormulaError> {
        parse_expression(source, kind, &BTreeMap::new())?.evaluate(|_| None)
    }
    #[test]
    fn precedence_units_and_exact_source() -> Result<(), FormulaError> {
        let source = "Fx = (#EntraxeA - 2mm ) / 5 * 3 * sin(#Angle1 - #Angle2 + 3deg)";
        let names = BTreeMap::from([
            ("EntraxeA".into(), ParameterId(1)),
            ("Angle1".into(), ParameterId(2)),
            ("Angle2".into(), ParameterId(3)),
        ]);
        let parsed = parse_expression(source, QuantityKind::Length, &names)?;
        assert_eq!(parsed.source(), source);
        let result = parsed.evaluate(|id| match id.0 {
            1 => Length::millimetres(12.).ok().map(Quantity::Length),
            2 => Angle::degrees(30.).ok().map(Quantity::Angle),
            3 => Angle::degrees(3.).ok().map(Quantity::Angle),
            _ => None,
        })?;
        assert!((raw(result) - 3.).abs() < 1e-12);
        assert_eq!(raw(evaluate("8/2*2", QuantityKind::Scalar)?), 8.);
        assert_eq!(raw(evaluate("-1mm + 3mm", QuantityKind::Length)?), 2.);
        assert_eq!(
            raw(evaluate("1.23456789", QuantityKind::Length)?),
            1.23456789
        );
        assert_eq!(raw(evaluate("#Depth := 1e1mm", QuantityKind::Length)?), 10.);
        Ok(())
    }
    #[test]
    fn rejects_invalid_dimensions_domains_grammar_and_bounds() {
        for source in [
            "1mm+1deg",
            "sin(1mm)",
            "1mm*2mm",
            "10*2",
            "(10)",
            "1/0",
            "tan(90deg)",
            "-1mm",
            "1e999mm",
            "1e-999mm",
            "1e-200mm*1e-200",
            "2^3",
            "2(3)",
            "min(1mm)",
            "abs(1,2)",
            "#missing",
            "wall",
            "1rad",
            "1,2",
            "Fx == 2mm",
        ] {
            assert!(evaluate(source, QuantityKind::Length).is_err(), "{source}");
        }
        for source in [
            format!("{}1mm{}", "(".repeat(17), ")".repeat(17)),
            format!("{}1mm", "-".repeat(17)),
            "1mm+".repeat(65),
            " ".repeat(257),
        ] {
            assert!(parse_expression(&source, QuantityKind::Length, &BTreeMap::new()).is_err());
        }
    }
    #[test]
    fn renamed_names_do_not_rebind_accepted_tokens() -> Result<(), FormulaError> {
        let original = BTreeMap::from([("A".into(), ParameterId(1))]);
        let parsed = parse_expression("#A/2", QuantityKind::Length, &original)?;
        let reused = BTreeMap::from([("A".into(), ParameterId(2)), ("B".into(), ParameterId(1))]);
        assert_eq!(parsed.references()[0].parameter, ParameterId(1));
        assert_eq!(
            parse_expression("#A/2", QuantityKind::Length, &reused)?.references()[0].parameter,
            ParameterId(2)
        );
        assert_eq!(
            parsed.evaluate(|_| None).map_err(|e| e.code),
            Err(FormulaErrorCode::MissingParameter)
        );
        Ok(())
    }
}

#[cfg(test)]
mod provenance_tests {
    use super::*;
    #[test]
    fn outside_edits_preserve_ids_but_retyped_tokens_bind_current_names() -> Result<(), FormulaError>
    {
        let old = parse_expression(
            "#A / 2",
            QuantityKind::Length,
            &BTreeMap::from([("A".into(), ParameterId(1))]),
        )?;
        let names = BTreeMap::from([("A".into(), ParameterId(2))]);
        let source = "#A / 3";
        let origins = vec![Some(0), Some(1), Some(2), Some(3), Some(4), None];
        assert_eq!(
            reparse_with_origins(&old, source, &origins, &names)?.references()[0].parameter,
            ParameterId(1)
        );
        let origins = vec![None, None, Some(2), Some(3), Some(4), Some(5)];
        assert_eq!(
            reparse_with_origins(&old, old.source(), &origins, &names)?.references()[0].parameter,
            ParameterId(2)
        );
        let no_old_name = BTreeMap::from([("B".into(), ParameterId(1))]);
        let edited = edit_expression(
            &old,
            &FormulaSourceEdit {
                replaced: SourceSpan { start: 5, end: 6 },
                inserted: "4".into(),
            },
            &no_old_name,
        )?;
        assert_eq!(edited.references()[0].parameter, ParameterId(1));
        assert!(reparse_with_origins(&old, source, &[Some(0); 6], &names).is_err());
        Ok(())
    }
}
