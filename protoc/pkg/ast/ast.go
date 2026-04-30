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

// Package ast defines the Abstract Syntax Tree for Tars IDL
package ast

// Document represents a complete Tars IDL file
type Document struct {
	FileName   string
	Includes   []string
	Namespaces []*Namespace
}

// Namespace represents a namespace in Tars IDL
type Namespace struct {
	Name       string
	Structs    []*Struct
	Enums      []*Enum
	Consts     []*Const
	Interfaces []*Interface
}

// BuiltinKind represents the kind of builtin type
type BuiltinKind int

const (
	KindVoid BuiltinKind = iota
	KindBool
	KindByte
	KindShort
	KindInt
	KindLong
	KindFloat
	KindDouble
	KindString
)

// Type represents a type in Tars IDL
type Type interface {
	TypeName() string
	IsSimple() bool
}

// BuiltinType represents a builtin type
type BuiltinType struct {
	Kind       BuiltinKind
	IsUnsigned bool
}

func (b *BuiltinType) TypeName() string {
	names := []string{"void", "bool", "byte", "short", "int", "long", "float", "double", "string"}
	if b.Kind >= 0 && int(b.Kind) < len(names) {
		prefix := ""
		if b.IsUnsigned {
			prefix = "unsigned "
		}
		return prefix + names[b.Kind]
	}
	return "unknown"
}

func (b *BuiltinType) IsSimple() bool {
	return b.Kind != KindVoid
}

// VectorType represents a vector<T> type
type VectorType struct {
	ElementType Type
	ArraySize   int  // 0 means not fixed-size array
	IsArray     bool // true if it's a fixed-size array
}

func (v *VectorType) TypeName() string {
	return "vector<" + v.ElementType.TypeName() + ">"
}

func (v *VectorType) IsSimple() bool {
	return false
}

// MapType represents a map<K, V> type
type MapType struct {
	KeyType   Type
	ValueType Type
}

func (m *MapType) TypeName() string {
	return "map<" + m.KeyType.TypeName() + ", " + m.ValueType.TypeName() + ">"
}

func (m *MapType) IsSimple() bool {
	return false
}

// CustomType represents a user-defined type (struct or enum)
type CustomType struct {
	Name      string // Full name including namespace (e.g., "Namespace::TypeName")
	Namespace string // Namespace part
	TypeName_ string // Just the type name
}

func (c *CustomType) TypeName() string {
	return c.Name
}

func (c *CustomType) IsSimple() bool {
	return false // Enums are simple but we'll handle that at code generation
}

// Field represents a field in a struct
type Field struct {
	Tag         int
	IsRequired  bool
	Type        Type
	Name        string
	HasDefault  bool
	DefaultValue string
}

// Struct represents a struct definition
type Struct struct {
	Name    string
	Fields  []*Field
	Keys    []string // For hash key definition
	SID     string   // Full scoped ID including namespace
}

// EnumMember represents a member of an enum
type EnumMember struct {
	Name       string
	HasValue   bool
	Value      int64
}

// Enum represents an enum definition
type Enum struct {
	Name    string
	Members []*EnumMember
	SID     string // Full scoped ID including namespace
}

// Const represents a constant definition
type Const struct {
	Type  Type
	Name  string
	Value string
}

// ParamDecl represents a parameter in a function
type ParamDecl struct {
	Type       Type
	Name       string
	IsOut      bool
	IsRouteKey bool
}

// Operation represents a function/method in an interface
type Operation struct {
	Name       string
	ReturnType Type
	Params     []*ParamDecl
}

// Interface represents an interface definition
type Interface struct {
	Name       string
	Operations []*Operation
}
