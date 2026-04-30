/**
 * Tencent is pleased to support the open source community by making Tars available.
 *
 * Copyright (C) 2016THL A29 Limited, a Tencent company. All rights reserved.
 *
 * Licensed under the BSD 3-Clause License (the "License"); you may not use this file except
 * in compliance with the License. You may obtain a copy of the License at
 *
 * https://opensource.org/licenses/BSD-3-Clause
 */

// Package lexer provides lexical analysis for Tars IDL
package lexer

import (
	"fmt"
	"strings"
	"unicode"
)

// TokenType represents the type of a token
type TokenType int

const (
	TOKEN_EOF TokenType = iota
	TOKEN_ERROR

	// Keywords
	TOKEN_MODULE
	TOKEN_NAMESPACE
	TOKEN_STRUCT
	TOKEN_INTERFACE
	TOKEN_ENUM
	TOKEN_CONST
	TOKEN_VOID
	TOKEN_BOOL
	TOKEN_BYTE
	TOKEN_SHORT
	TOKEN_INT
	TOKEN_LONG
	TOKEN_FLOAT
	TOKEN_DOUBLE
	TOKEN_STRING
	TOKEN_VECTOR
	TOKEN_MAP
	TOKEN_REQUIRE
	TOKEN_OPTIONAL
	TOKEN_OUT
	TOKEN_ROUTEKEY
	TOKEN_KEY
	TOKEN_TRUE
	TOKEN_FALSE
	TOKEN_UNSIGNED
	TOKEN_INCLUDE

	// Literals
	TOKEN_IDENTIFIER
	TOKEN_INTEGER
	TOKEN_FLOAT_LITERAL
	TOKEN_STRING_LITERAL

	// Operators and punctuation
	TOKEN_LBRACE      // {
	TOKEN_RBRACE      // }
	TOKEN_LPAREN      // (
	TOKEN_RPAREN      // )
	TOKEN_LBRACKET    // [
	TOKEN_RBRACKET    // ]
	TOKEN_LANGLE      // <
	TOKEN_RANGLE      // >
	TOKEN_SEMICOLON   // ;
	TOKEN_COMMA       // ,
	TOKEN_COLON       // :
	TOKEN_EQUALS      // =
	TOKEN_SCOPE       // ::
	TOKEN_ASTERISK    // *
)

var tokenNames = map[TokenType]string{
	TOKEN_EOF:            "EOF",
	TOKEN_ERROR:          "ERROR",
	TOKEN_MODULE:         "module",
	TOKEN_NAMESPACE:      "namespace",
	TOKEN_STRUCT:         "struct",
	TOKEN_INTERFACE:      "interface",
	TOKEN_ENUM:           "enum",
	TOKEN_CONST:          "const",
	TOKEN_VOID:           "void",
	TOKEN_BOOL:           "bool",
	TOKEN_BYTE:           "byte",
	TOKEN_SHORT:          "short",
	TOKEN_INT:            "int",
	TOKEN_LONG:           "long",
	TOKEN_FLOAT:          "float",
	TOKEN_DOUBLE:         "double",
	TOKEN_STRING:         "string",
	TOKEN_VECTOR:         "vector",
	TOKEN_MAP:            "map",
	TOKEN_REQUIRE:        "require",
	TOKEN_OPTIONAL:       "optional",
	TOKEN_OUT:            "out",
	TOKEN_ROUTEKEY:       "routekey",
	TOKEN_KEY:            "key",
	TOKEN_TRUE:           "true",
	TOKEN_FALSE:          "false",
	TOKEN_UNSIGNED:       "unsigned",
	TOKEN_INCLUDE:        "#include",
	TOKEN_IDENTIFIER:     "IDENTIFIER",
	TOKEN_INTEGER:        "INTEGER",
	TOKEN_FLOAT_LITERAL:  "FLOAT",
	TOKEN_STRING_LITERAL: "STRING",
	TOKEN_LBRACE:         "{",
	TOKEN_RBRACE:         "}",
	TOKEN_LPAREN:         "(",
	TOKEN_RPAREN:         ")",
	TOKEN_LBRACKET:       "[",
	TOKEN_RBRACKET:       "]",
	TOKEN_LANGLE:         "<",
	TOKEN_RANGLE:         ">",
	TOKEN_SEMICOLON:      ";",
	TOKEN_COMMA:          ",",
	TOKEN_COLON:          ":",
	TOKEN_EQUALS:         "=",
	TOKEN_SCOPE:          "::",
	TOKEN_ASTERISK:       "*",
}

var keywords = map[string]TokenType{
	"module":    TOKEN_MODULE,
	"namespace": TOKEN_NAMESPACE,
	"struct":    TOKEN_STRUCT,
	"interface": TOKEN_INTERFACE,
	"enum":      TOKEN_ENUM,
	"const":     TOKEN_CONST,
	"void":      TOKEN_VOID,
	"bool":      TOKEN_BOOL,
	"byte":      TOKEN_BYTE,
	"short":     TOKEN_SHORT,
	"int":       TOKEN_INT,
	"long":      TOKEN_LONG,
	"float":     TOKEN_FLOAT,
	"double":    TOKEN_DOUBLE,
	"string":    TOKEN_STRING,
	"vector":    TOKEN_VECTOR,
	"map":       TOKEN_MAP,
	"require":   TOKEN_REQUIRE,
	"optional":  TOKEN_OPTIONAL,
	"out":       TOKEN_OUT,
	"routekey":  TOKEN_ROUTEKEY,
	"key":       TOKEN_KEY,
	"true":      TOKEN_TRUE,
	"false":     TOKEN_FALSE,
	"unsigned":  TOKEN_UNSIGNED,
}

// Token represents a lexical token
type Token struct {
	Type    TokenType
	Value   string
	Line    int
	Column  int
}

func (t Token) String() string {
	if name, ok := tokenNames[t.Type]; ok {
		if t.Value != "" && t.Type != TOKEN_EOF {
			return fmt.Sprintf("%s(%s) at %d:%d", name, t.Value, t.Line, t.Column)
		}
		return fmt.Sprintf("%s at %d:%d", name, t.Line, t.Column)
	}
	return fmt.Sprintf("UNKNOWN(%d) at %d:%d", t.Type, t.Line, t.Column)
}

// Lexer performs lexical analysis on Tars IDL source
type Lexer struct {
	input   string
	pos     int
	line    int
	column  int
	tokens  []Token
}

// NewLexer creates a new Lexer for the given input
func NewLexer(input string) *Lexer {
	return &Lexer{
		input:  input,
		pos:    0,
		line:   1,
		column: 1,
	}
}

// Tokenize returns all tokens from the input
func (l *Lexer) Tokenize() ([]Token, error) {
	for {
		token := l.nextToken()
		l.tokens = append(l.tokens, token)
		if token.Type == TOKEN_EOF {
			break
		}
		if token.Type == TOKEN_ERROR {
			return nil, fmt.Errorf("lexer error at line %d, column %d: %s", token.Line, token.Column, token.Value)
		}
	}
	return l.tokens, nil
}

func (l *Lexer) nextToken() Token {
	l.skipWhitespaceAndComments()

	if l.pos >= len(l.input) {
		return Token{Type: TOKEN_EOF, Line: l.line, Column: l.column}
	}

	startLine := l.line
	startColumn := l.column

	ch := l.input[l.pos]

	// Check for #include
	if ch == '#' {
		if l.matchKeyword("#include") {
			l.advance(8)
			return Token{Type: TOKEN_INCLUDE, Value: "#include", Line: startLine, Column: startColumn}
		}
	}

	// Single character tokens
	switch ch {
	case '{':
		l.advance(1)
		return Token{Type: TOKEN_LBRACE, Value: "{", Line: startLine, Column: startColumn}
	case '}':
		l.advance(1)
		return Token{Type: TOKEN_RBRACE, Value: "}", Line: startLine, Column: startColumn}
	case '(':
		l.advance(1)
		return Token{Type: TOKEN_LPAREN, Value: "(", Line: startLine, Column: startColumn}
	case ')':
		l.advance(1)
		return Token{Type: TOKEN_RPAREN, Value: ")", Line: startLine, Column: startColumn}
	case '[':
		l.advance(1)
		return Token{Type: TOKEN_LBRACKET, Value: "[", Line: startLine, Column: startColumn}
	case ']':
		l.advance(1)
		return Token{Type: TOKEN_RBRACKET, Value: "]", Line: startLine, Column: startColumn}
	case '<':
		l.advance(1)
		return Token{Type: TOKEN_LANGLE, Value: "<", Line: startLine, Column: startColumn}
	case '>':
		l.advance(1)
		return Token{Type: TOKEN_RANGLE, Value: ">", Line: startLine, Column: startColumn}
	case ';':
		l.advance(1)
		return Token{Type: TOKEN_SEMICOLON, Value: ";", Line: startLine, Column: startColumn}
	case ',':
		l.advance(1)
		return Token{Type: TOKEN_COMMA, Value: ",", Line: startLine, Column: startColumn}
	case '=':
		l.advance(1)
		return Token{Type: TOKEN_EQUALS, Value: "=", Line: startLine, Column: startColumn}
	case '*':
		l.advance(1)
		return Token{Type: TOKEN_ASTERISK, Value: "*", Line: startLine, Column: startColumn}
	case ':':
		if l.pos+1 < len(l.input) && l.input[l.pos+1] == ':' {
			l.advance(2)
			return Token{Type: TOKEN_SCOPE, Value: "::", Line: startLine, Column: startColumn}
		}
		l.advance(1)
		return Token{Type: TOKEN_COLON, Value: ":", Line: startLine, Column: startColumn}
	case '"':
		return l.readString()
	}

	// Numbers
	if unicode.IsDigit(rune(ch)) || (ch == '-' && l.pos+1 < len(l.input) && unicode.IsDigit(rune(l.input[l.pos+1]))) {
		return l.readNumber()
	}

	// Identifiers and keywords
	if unicode.IsLetter(rune(ch)) || ch == '_' {
		return l.readIdentifier()
	}

	l.advance(1)
	return Token{Type: TOKEN_ERROR, Value: fmt.Sprintf("unexpected character: %c", ch), Line: startLine, Column: startColumn}
}

func (l *Lexer) advance(n int) {
	for i := 0; i < n && l.pos < len(l.input); i++ {
		if l.input[l.pos] == '\n' {
			l.line++
			l.column = 1
		} else {
			l.column++
		}
		l.pos++
	}
}

func (l *Lexer) skipWhitespaceAndComments() {
	for l.pos < len(l.input) {
		ch := l.input[l.pos]

		// Skip whitespace
		if unicode.IsSpace(rune(ch)) {
			l.advance(1)
			continue
		}

		// Skip single-line comments
		if ch == '/' && l.pos+1 < len(l.input) && l.input[l.pos+1] == '/' {
			l.advance(2)
			for l.pos < len(l.input) && l.input[l.pos] != '\n' {
				l.advance(1)
			}
			continue
		}

		// Skip multi-line comments
		if ch == '/' && l.pos+1 < len(l.input) && l.input[l.pos+1] == '*' {
			l.advance(2)
			for l.pos+1 < len(l.input) {
				if l.input[l.pos] == '*' && l.input[l.pos+1] == '/' {
					l.advance(2)
					break
				}
				l.advance(1)
			}
			continue
		}

		break
	}
}

func (l *Lexer) matchKeyword(keyword string) bool {
	if l.pos+len(keyword) > len(l.input) {
		return false
	}
	return l.input[l.pos:l.pos+len(keyword)] == keyword
}

func (l *Lexer) readString() Token {
	startLine := l.line
	startColumn := l.column
	l.advance(1) // Skip opening quote

	var sb strings.Builder
	for l.pos < len(l.input) {
		ch := l.input[l.pos]
		if ch == '"' {
			l.advance(1)
			return Token{Type: TOKEN_STRING_LITERAL, Value: sb.String(), Line: startLine, Column: startColumn}
		}
		if ch == '\\' && l.pos+1 < len(l.input) {
			l.advance(1)
			escaped := l.input[l.pos]
			switch escaped {
			case 'n':
				sb.WriteByte('\n')
			case 't':
				sb.WriteByte('\t')
			case 'r':
				sb.WriteByte('\r')
			case '\\':
				sb.WriteByte('\\')
			case '"':
				sb.WriteByte('"')
			default:
				sb.WriteByte(escaped)
			}
			l.advance(1)
			continue
		}
		if ch == '\n' {
			return Token{Type: TOKEN_ERROR, Value: "unterminated string literal", Line: startLine, Column: startColumn}
		}
		sb.WriteByte(ch)
		l.advance(1)
	}
	return Token{Type: TOKEN_ERROR, Value: "unterminated string literal", Line: startLine, Column: startColumn}
}

func (l *Lexer) readNumber() Token {
	startLine := l.line
	startColumn := l.column
	startPos := l.pos

	// Handle negative sign
	if l.input[l.pos] == '-' {
		l.advance(1)
	}

	// Handle hex numbers
	if l.pos+1 < len(l.input) && l.input[l.pos] == '0' && (l.input[l.pos+1] == 'x' || l.input[l.pos+1] == 'X') {
		l.advance(2)
		for l.pos < len(l.input) && isHexDigit(l.input[l.pos]) {
			l.advance(1)
		}
		return Token{Type: TOKEN_INTEGER, Value: l.input[startPos:l.pos], Line: startLine, Column: startColumn}
	}

	// Read integer part
	for l.pos < len(l.input) && unicode.IsDigit(rune(l.input[l.pos])) {
		l.advance(1)
	}

	// Check for float
	isFloat := false
	if l.pos < len(l.input) && l.input[l.pos] == '.' {
		l.advance(1)
		isFloat = true
		for l.pos < len(l.input) && unicode.IsDigit(rune(l.input[l.pos])) {
			l.advance(1)
		}
	}

	// Check for exponent
	if l.pos < len(l.input) && (l.input[l.pos] == 'e' || l.input[l.pos] == 'E') {
		l.advance(1)
		isFloat = true
		if l.pos < len(l.input) && (l.input[l.pos] == '+' || l.input[l.pos] == '-') {
			l.advance(1)
		}
		for l.pos < len(l.input) && unicode.IsDigit(rune(l.input[l.pos])) {
			l.advance(1)
		}
	}

	value := l.input[startPos:l.pos]
	if isFloat {
		return Token{Type: TOKEN_FLOAT_LITERAL, Value: value, Line: startLine, Column: startColumn}
	}
	return Token{Type: TOKEN_INTEGER, Value: value, Line: startLine, Column: startColumn}
}

func (l *Lexer) readIdentifier() Token {
	startLine := l.line
	startColumn := l.column
	startPos := l.pos

	for l.pos < len(l.input) {
		ch := l.input[l.pos]
		if unicode.IsLetter(rune(ch)) || unicode.IsDigit(rune(ch)) || ch == '_' {
			l.advance(1)
		} else {
			break
		}
	}

	value := l.input[startPos:l.pos]
	if tokenType, ok := keywords[value]; ok {
		return Token{Type: tokenType, Value: value, Line: startLine, Column: startColumn}
	}
	return Token{Type: TOKEN_IDENTIFIER, Value: value, Line: startLine, Column: startColumn}
}

func isHexDigit(ch byte) bool {
	return unicode.IsDigit(rune(ch)) || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F')
}
