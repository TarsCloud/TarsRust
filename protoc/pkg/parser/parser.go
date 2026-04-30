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

// Package parser provides parsing for Tars IDL
package parser

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/ast"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/lexer"
)

// Parser parses Tars IDL files
type Parser struct {
	tokens       []lexer.Token
	pos          int
	fileName     string
	includePaths []string
	document     *ast.Document
	currentNs    *ast.Namespace
	// Type registry for resolving custom types
	structs map[string]*ast.Struct
	enums   map[string]*ast.Enum
}

// NewParser creates a new Parser
func NewParser(input string, fileName string, includePaths []string) *Parser {
	return &Parser{
		fileName:     fileName,
		includePaths: includePaths,
		structs:      make(map[string]*ast.Struct),
		enums:        make(map[string]*ast.Enum),
	}
}

// Parse parses the input and returns the AST
func (p *Parser) Parse() (*ast.Document, error) {
	// Read file content
	content, err := os.ReadFile(p.fileName)
	if err != nil {
		return nil, fmt.Errorf("failed to read file %s: %w", p.fileName, err)
	}

	return p.ParseString(string(content))
}

// ParseString parses a string input
func (p *Parser) ParseString(input string) (*ast.Document, error) {
	l := lexer.NewLexer(input)
	tokens, err := l.Tokenize()
	if err != nil {
		return nil, err
	}
	p.tokens = tokens
	p.pos = 0

	p.document = &ast.Document{
		FileName: p.fileName,
	}

	for !p.isAtEnd() {
		if err := p.parseTopLevel(); err != nil {
			return nil, err
		}
	}

	return p.document, nil
}

func (p *Parser) parseTopLevel() error {
	token := p.current()

	switch token.Type {
	case lexer.TOKEN_INCLUDE:
		return p.parseInclude()
	case lexer.TOKEN_MODULE, lexer.TOKEN_NAMESPACE:
		return p.parseNamespace()
	case lexer.TOKEN_EOF:
		return nil
	default:
		return p.error("expected namespace, module, or #include, got %s", token)
	}
}

func (p *Parser) parseInclude() error {
	p.advance() // consume #include

	if !p.check(lexer.TOKEN_STRING_LITERAL) {
		return p.error("expected string literal after #include")
	}

	includePath := p.current().Value
	p.advance()

	p.document.Includes = append(p.document.Includes, includePath)

	// Try to parse the included file
	resolvedPath := p.resolveIncludePath(includePath)
	if resolvedPath != "" {
		content, err := os.ReadFile(resolvedPath)
		if err == nil {
			subParser := NewParser(string(content), resolvedPath, p.includePaths)
			subParser.structs = p.structs
			subParser.enums = p.enums
			_, _ = subParser.ParseString(string(content))
			// Merge type registries
			for k, v := range subParser.structs {
				p.structs[k] = v
			}
			for k, v := range subParser.enums {
				p.enums[k] = v
			}
		}
	}

	return nil
}

func (p *Parser) resolveIncludePath(includePath string) string {
	// Remove quotes if present
	includePath = strings.Trim(includePath, "\"")

	// Try each include path
	for _, basePath := range p.includePaths {
		fullPath := filepath.Join(basePath, includePath)
		if _, err := os.Stat(fullPath); err == nil {
			return fullPath
		}
	}

	// Try current directory
	if _, err := os.Stat(includePath); err == nil {
		return includePath
	}

	return ""
}

func (p *Parser) parseNamespace() error {
	p.advance() // consume 'namespace' or 'module'

	if !p.check(lexer.TOKEN_IDENTIFIER) {
		return p.error("expected namespace name")
	}

	name := p.current().Value
	p.advance()

	if !p.consume(lexer.TOKEN_LBRACE) {
		return p.error("expected '{' after namespace name")
	}

	ns := &ast.Namespace{Name: name}
	p.currentNs = ns

	for !p.check(lexer.TOKEN_RBRACE) && !p.isAtEnd() {
		if err := p.parseNamespaceContent(); err != nil {
			return err
		}
	}

	if !p.consume(lexer.TOKEN_RBRACE) {
		return p.error("expected '}' at end of namespace")
	}

	// Optional semicolon after namespace
	p.consume(lexer.TOKEN_SEMICOLON)

	p.document.Namespaces = append(p.document.Namespaces, ns)
	p.currentNs = nil

	return nil
}

func (p *Parser) parseNamespaceContent() error {
	token := p.current()

	switch token.Type {
	case lexer.TOKEN_STRUCT:
		return p.parseStruct()
	case lexer.TOKEN_ENUM:
		return p.parseEnum()
	case lexer.TOKEN_CONST:
		return p.parseConst()
	case lexer.TOKEN_INTERFACE:
		return p.parseInterface()
	case lexer.TOKEN_KEY:
		return p.parseKey()
	case lexer.TOKEN_SEMICOLON:
		p.advance()
		return nil
	default:
		return p.error("unexpected token in namespace: %s", token)
	}
}

func (p *Parser) parseStruct() error {
	p.advance() // consume 'struct'

	if !p.check(lexer.TOKEN_IDENTIFIER) {
		return p.error("expected struct name")
	}

	name := p.current().Value
	p.advance()

	if !p.consume(lexer.TOKEN_LBRACE) {
		return p.error("expected '{' after struct name")
	}

	s := &ast.Struct{
		Name: name,
		SID:  p.currentNs.Name + "::" + name,
	}

	for !p.check(lexer.TOKEN_RBRACE) && !p.isAtEnd() {
		field, err := p.parseField()
		if err != nil {
			return err
		}
		if field != nil {
			s.Fields = append(s.Fields, field)
		}
	}

	if !p.consume(lexer.TOKEN_RBRACE) {
		return p.error("expected '}' at end of struct")
	}

	p.consume(lexer.TOKEN_SEMICOLON)

	p.currentNs.Structs = append(p.currentNs.Structs, s)
	p.structs[s.SID] = s
	p.structs[name] = s

	return nil
}

func (p *Parser) parseField() (*ast.Field, error) {
	// Field format: tag require/optional type name [= default];
	if !p.check(lexer.TOKEN_INTEGER) {
		if p.check(lexer.TOKEN_SEMICOLON) {
			p.advance()
			return nil, nil
		}
		return nil, p.error("expected field tag number")
	}

	tag, err := strconv.Atoi(p.current().Value)
	if err != nil {
		return nil, p.error("invalid tag number: %s", p.current().Value)
	}
	p.advance()

	// require or optional
	isRequired := true
	if p.check(lexer.TOKEN_REQUIRE) {
		p.advance()
		isRequired = true
	} else if p.check(lexer.TOKEN_OPTIONAL) {
		p.advance()
		isRequired = false
	} else {
		return nil, p.error("expected 'require' or 'optional'")
	}

	// Type
	fieldType, err := p.parseType()
	if err != nil {
		return nil, err
	}

	// Name
	if !p.check(lexer.TOKEN_IDENTIFIER) {
		return nil, p.error("expected field name")
	}
	name := p.current().Value
	p.advance()

	// Check for array syntax: name[size]
	if p.check(lexer.TOKEN_LBRACKET) {
		p.advance()
		if !p.check(lexer.TOKEN_INTEGER) {
			return nil, p.error("expected array size")
		}
		size, _ := strconv.Atoi(p.current().Value)
		p.advance()
		if !p.consume(lexer.TOKEN_RBRACKET) {
			return nil, p.error("expected ']'")
		}
		fieldType = &ast.VectorType{
			ElementType: fieldType,
			ArraySize:   size,
			IsArray:     true,
		}
	}

	field := &ast.Field{
		Tag:        tag,
		IsRequired: isRequired,
		Type:       fieldType,
		Name:       name,
	}

	// Optional default value
	if p.check(lexer.TOKEN_EQUALS) {
		p.advance()
		defaultValue, err := p.parseConstValue()
		if err != nil {
			return nil, err
		}
		field.HasDefault = true
		field.DefaultValue = defaultValue
	}

	if !p.consume(lexer.TOKEN_SEMICOLON) {
		return nil, p.error("expected ';' after field")
	}

	return field, nil
}

func (p *Parser) parseType() (ast.Type, error) {
	// Check for unsigned
	isUnsigned := false
	if p.check(lexer.TOKEN_UNSIGNED) {
		isUnsigned = true
		p.advance()
	}

	token := p.current()

	switch token.Type {
	case lexer.TOKEN_VOID:
		p.advance()
		return &ast.BuiltinType{Kind: ast.KindVoid}, nil
	case lexer.TOKEN_BOOL:
		p.advance()
		return &ast.BuiltinType{Kind: ast.KindBool, IsUnsigned: isUnsigned}, nil
	case lexer.TOKEN_BYTE:
		p.advance()
		if isUnsigned {
			return &ast.BuiltinType{Kind: ast.KindShort, IsUnsigned: true}, nil
		}
		return &ast.BuiltinType{Kind: ast.KindByte, IsUnsigned: isUnsigned}, nil
	case lexer.TOKEN_SHORT:
		p.advance()
		if isUnsigned {
			return &ast.BuiltinType{Kind: ast.KindInt, IsUnsigned: true}, nil
		}
		return &ast.BuiltinType{Kind: ast.KindShort, IsUnsigned: isUnsigned}, nil
	case lexer.TOKEN_INT:
		p.advance()
		if isUnsigned {
			return &ast.BuiltinType{Kind: ast.KindLong, IsUnsigned: true}, nil
		}
		return &ast.BuiltinType{Kind: ast.KindInt, IsUnsigned: isUnsigned}, nil
	case lexer.TOKEN_LONG:
		p.advance()
		return &ast.BuiltinType{Kind: ast.KindLong, IsUnsigned: isUnsigned}, nil
	case lexer.TOKEN_FLOAT:
		p.advance()
		return &ast.BuiltinType{Kind: ast.KindFloat, IsUnsigned: isUnsigned}, nil
	case lexer.TOKEN_DOUBLE:
		p.advance()
		return &ast.BuiltinType{Kind: ast.KindDouble, IsUnsigned: isUnsigned}, nil
	case lexer.TOKEN_STRING:
		p.advance()
		return &ast.BuiltinType{Kind: ast.KindString}, nil
	case lexer.TOKEN_VECTOR:
		return p.parseVectorType()
	case lexer.TOKEN_MAP:
		return p.parseMapType()
	case lexer.TOKEN_IDENTIFIER:
		return p.parseCustomType()
	default:
		return nil, p.error("expected type, got %s", token)
	}
}

func (p *Parser) parseVectorType() (ast.Type, error) {
	p.advance() // consume 'vector'

	if !p.consume(lexer.TOKEN_LANGLE) {
		return nil, p.error("expected '<' after vector")
	}

	elemType, err := p.parseType()
	if err != nil {
		return nil, err
	}

	if !p.consume(lexer.TOKEN_RANGLE) {
		return nil, p.error("expected '>' after vector element type")
	}

	return &ast.VectorType{ElementType: elemType}, nil
}

func (p *Parser) parseMapType() (ast.Type, error) {
	p.advance() // consume 'map'

	if !p.consume(lexer.TOKEN_LANGLE) {
		return nil, p.error("expected '<' after map")
	}

	keyType, err := p.parseType()
	if err != nil {
		return nil, err
	}

	if !p.consume(lexer.TOKEN_COMMA) {
		return nil, p.error("expected ',' in map type")
	}

	valueType, err := p.parseType()
	if err != nil {
		return nil, err
	}

	if !p.consume(lexer.TOKEN_RANGLE) {
		return nil, p.error("expected '>' after map value type")
	}

	return &ast.MapType{KeyType: keyType, ValueType: valueType}, nil
}

func (p *Parser) parseCustomType() (ast.Type, error) {
	var parts []string
	parts = append(parts, p.current().Value)
	p.advance()

	for p.check(lexer.TOKEN_SCOPE) {
		p.advance()
		if !p.check(lexer.TOKEN_IDENTIFIER) {
			return nil, p.error("expected identifier after '::'")
		}
		parts = append(parts, p.current().Value)
		p.advance()
	}

	fullName := strings.Join(parts, "::")
	typeName := parts[len(parts)-1]
	namespace := ""
	if len(parts) > 1 {
		namespace = strings.Join(parts[:len(parts)-1], "::")
	}

	return &ast.CustomType{
		Name:      fullName,
		Namespace: namespace,
		TypeName_: typeName,
	}, nil
}

func (p *Parser) parseEnum() error {
	p.advance() // consume 'enum'

	if !p.check(lexer.TOKEN_IDENTIFIER) {
		return p.error("expected enum name")
	}

	name := p.current().Value
	p.advance()

	if !p.consume(lexer.TOKEN_LBRACE) {
		return p.error("expected '{' after enum name")
	}

	e := &ast.Enum{
		Name: name,
		SID:  p.currentNs.Name + "::" + name,
	}

	var nextValue int64 = 0
	for !p.check(lexer.TOKEN_RBRACE) && !p.isAtEnd() {
		if !p.check(lexer.TOKEN_IDENTIFIER) {
			if p.check(lexer.TOKEN_COMMA) {
				p.advance()
				continue
			}
			break
		}

		memberName := p.current().Value
		p.advance()

		member := &ast.EnumMember{Name: memberName}

		if p.check(lexer.TOKEN_EQUALS) {
			p.advance()
			if !p.check(lexer.TOKEN_INTEGER) {
				return p.error("expected integer value for enum member")
			}
			val, _ := strconv.ParseInt(p.current().Value, 0, 64)
			p.advance()
			member.HasValue = true
			member.Value = val
			nextValue = val + 1
		} else {
			member.HasValue = true
			member.Value = nextValue
			nextValue++
		}

		e.Members = append(e.Members, member)

		if !p.check(lexer.TOKEN_COMMA) {
			break
		}
		p.advance()
	}

	if !p.consume(lexer.TOKEN_RBRACE) {
		return p.error("expected '}' at end of enum")
	}

	p.consume(lexer.TOKEN_SEMICOLON)

	p.currentNs.Enums = append(p.currentNs.Enums, e)
	p.enums[e.SID] = e
	p.enums[name] = e

	return nil
}

func (p *Parser) parseConst() error {
	p.advance() // consume 'const'

	constType, err := p.parseType()
	if err != nil {
		return err
	}

	if !p.check(lexer.TOKEN_IDENTIFIER) {
		return p.error("expected const name")
	}
	name := p.current().Value
	p.advance()

	if !p.consume(lexer.TOKEN_EQUALS) {
		return p.error("expected '=' after const name")
	}

	value, err := p.parseConstValue()
	if err != nil {
		return err
	}

	if !p.consume(lexer.TOKEN_SEMICOLON) {
		return p.error("expected ';' after const")
	}

	c := &ast.Const{
		Type:  constType,
		Name:  name,
		Value: value,
	}

	p.currentNs.Consts = append(p.currentNs.Consts, c)

	return nil
}

func (p *Parser) parseConstValue() (string, error) {
	token := p.current()

	switch token.Type {
	case lexer.TOKEN_INTEGER, lexer.TOKEN_FLOAT_LITERAL:
		p.advance()
		return token.Value, nil
	case lexer.TOKEN_STRING_LITERAL:
		p.advance()
		return "\"" + token.Value + "\"", nil
	case lexer.TOKEN_TRUE:
		p.advance()
		return "true", nil
	case lexer.TOKEN_FALSE:
		p.advance()
		return "false", nil
	case lexer.TOKEN_IDENTIFIER:
		// Could be enum value
		var parts []string
		parts = append(parts, p.current().Value)
		p.advance()
		for p.check(lexer.TOKEN_SCOPE) {
			p.advance()
			if p.check(lexer.TOKEN_IDENTIFIER) {
				parts = append(parts, p.current().Value)
				p.advance()
			}
		}
		return strings.Join(parts, "::"), nil
	default:
		return "", p.error("expected constant value")
	}
}

func (p *Parser) parseInterface() error {
	p.advance() // consume 'interface'

	if !p.check(lexer.TOKEN_IDENTIFIER) {
		return p.error("expected interface name")
	}

	name := p.current().Value
	p.advance()

	if !p.consume(lexer.TOKEN_LBRACE) {
		return p.error("expected '{' after interface name")
	}

	iface := &ast.Interface{Name: name}

	for !p.check(lexer.TOKEN_RBRACE) && !p.isAtEnd() {
		op, err := p.parseOperation()
		if err != nil {
			return err
		}
		if op != nil {
			iface.Operations = append(iface.Operations, op)
		}
	}

	if !p.consume(lexer.TOKEN_RBRACE) {
		return p.error("expected '}' at end of interface")
	}

	p.consume(lexer.TOKEN_SEMICOLON)

	p.currentNs.Interfaces = append(p.currentNs.Interfaces, iface)

	return nil
}

func (p *Parser) parseOperation() (*ast.Operation, error) {
	if p.check(lexer.TOKEN_SEMICOLON) {
		p.advance()
		return nil, nil
	}

	// Return type
	returnType, err := p.parseType()
	if err != nil {
		return nil, err
	}

	// Operation name
	if !p.check(lexer.TOKEN_IDENTIFIER) {
		return nil, p.error("expected operation name")
	}
	name := p.current().Value
	p.advance()

	if !p.consume(lexer.TOKEN_LPAREN) {
		return nil, p.error("expected '(' after operation name")
	}

	op := &ast.Operation{
		Name:       name,
		ReturnType: returnType,
	}

	// Parameters
	for !p.check(lexer.TOKEN_RPAREN) && !p.isAtEnd() {
		param, err := p.parseParam()
		if err != nil {
			return nil, err
		}
		op.Params = append(op.Params, param)

		if !p.check(lexer.TOKEN_COMMA) {
			break
		}
		p.advance()
	}

	if !p.consume(lexer.TOKEN_RPAREN) {
		return nil, p.error("expected ')' after parameters")
	}

	if !p.consume(lexer.TOKEN_SEMICOLON) {
		return nil, p.error("expected ';' after operation")
	}

	return op, nil
}

func (p *Parser) parseParam() (*ast.ParamDecl, error) {
	isOut := false
	isRouteKey := false

	if p.check(lexer.TOKEN_OUT) {
		isOut = true
		p.advance()
	} else if p.check(lexer.TOKEN_ROUTEKEY) {
		isRouteKey = true
		p.advance()
	}

	paramType, err := p.parseType()
	if err != nil {
		return nil, err
	}

	if !p.check(lexer.TOKEN_IDENTIFIER) {
		return nil, p.error("expected parameter name")
	}
	name := p.current().Value
	p.advance()

	return &ast.ParamDecl{
		Type:       paramType,
		Name:       name,
		IsOut:      isOut,
		IsRouteKey: isRouteKey,
	}, nil
}

func (p *Parser) parseKey() error {
	p.advance() // consume 'key'

	if !p.consume(lexer.TOKEN_LBRACKET) {
		return p.error("expected '[' after key")
	}

	// struct name
	if !p.check(lexer.TOKEN_IDENTIFIER) {
		return p.error("expected struct name in key")
	}
	structName := p.current().Value
	p.advance()

	// Handle scoped names
	for p.check(lexer.TOKEN_SCOPE) {
		p.advance()
		if p.check(lexer.TOKEN_IDENTIFIER) {
			structName += "::" + p.current().Value
			p.advance()
		}
	}

	if !p.consume(lexer.TOKEN_COMMA) {
		return p.error("expected ',' after struct name in key")
	}

	// Key fields
	var keys []string
	for !p.check(lexer.TOKEN_RBRACKET) && !p.isAtEnd() {
		if !p.check(lexer.TOKEN_IDENTIFIER) {
			return p.error("expected field name in key")
		}
		keys = append(keys, p.current().Value)
		p.advance()

		if !p.check(lexer.TOKEN_COMMA) {
			break
		}
		p.advance()
	}

	if !p.consume(lexer.TOKEN_RBRACKET) {
		return p.error("expected ']' at end of key")
	}

	p.consume(lexer.TOKEN_SEMICOLON)

	// Find and update the struct
	if s, ok := p.structs[structName]; ok {
		s.Keys = keys
	}

	return nil
}

// Helper methods

func (p *Parser) current() lexer.Token {
	if p.pos >= len(p.tokens) {
		return lexer.Token{Type: lexer.TOKEN_EOF}
	}
	return p.tokens[p.pos]
}

func (p *Parser) advance() {
	if p.pos < len(p.tokens) {
		p.pos++
	}
}

func (p *Parser) check(t lexer.TokenType) bool {
	return p.current().Type == t
}

func (p *Parser) consume(t lexer.TokenType) bool {
	if p.check(t) {
		p.advance()
		return true
	}
	return false
}

func (p *Parser) isAtEnd() bool {
	return p.current().Type == lexer.TOKEN_EOF
}

func (p *Parser) error(format string, args ...interface{}) error {
	token := p.current()
	msg := fmt.Sprintf(format, args...)
	return fmt.Errorf("%s:%d:%d: %s", p.fileName, token.Line, token.Column, msg)
}
