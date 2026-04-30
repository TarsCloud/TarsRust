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

// Package typescript generates TypeScript code from Tars IDL
package typescript

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/ast"
)

// Generator generates TypeScript code
type Generator struct {
	baseDir string
	enums   map[string]bool
}

// NewGenerator creates a new TypeScript generator
func NewGenerator(baseDir string) *Generator {
	return &Generator{
		baseDir: baseDir,
		enums:   make(map[string]bool),
	}
}

// Generate generates TypeScript code for the document
func (g *Generator) Generate(doc *ast.Document, baseName string) error {
	// First pass: collect all enums
	for _, ns := range doc.Namespaces {
		for _, e := range ns.Enums {
			g.enums[e.Name] = true
			g.enums[ns.Name+"::"+e.Name] = true
		}
	}

	for _, ns := range doc.Namespaces {
		if err := g.generateNamespace(ns, baseName); err != nil {
			return err
		}
	}
	return nil
}

func (g *Generator) generateNamespace(ns *ast.Namespace, baseName string) error {
	// Generate types file (interfaces, enums, structs)
	if err := g.generateTypesFile(ns, baseName); err != nil {
		return err
	}

	// Generate client proxy file
	if len(ns.Interfaces) > 0 {
		if err := g.generateClientFile(ns, baseName); err != nil {
			return err
		}
		// Generate server interface file
		if err := g.generateServerFile(ns, baseName); err != nil {
			return err
		}
	}

	// Generate index file
	if err := g.generateIndexFile(ns, baseName); err != nil {
		return err
	}

	return nil
}

func (g *Generator) generateTypesFile(ns *ast.Namespace, baseName string) error {
	var sb strings.Builder

	g.printHeader(&sb)

	// Imports
	sb.WriteString("import { TarsInputStream, TarsOutputStream } from '@aspect/tars-stream';\n\n")

	// Generate constants
	if len(ns.Consts) > 0 {
		sb.WriteString("// ============ Constants ============\n\n")
		for _, c := range ns.Consts {
			tsType := g.toTSType(c.Type)
			value := c.Value
			sb.WriteString(fmt.Sprintf("export const %s: %s = %s;\n", c.Name, tsType, value))
		}
		sb.WriteString("\n")
	}

	// Generate enums
	if len(ns.Enums) > 0 {
		sb.WriteString("// ============ Enums ============\n\n")
		for _, e := range ns.Enums {
			g.generateEnum(&sb, e)
			sb.WriteString("\n")
		}
	}

	// Generate struct interfaces and classes
	if len(ns.Structs) > 0 {
		sb.WriteString("// ============ Structs ============\n\n")
		for _, s := range ns.Structs {
			g.generateStruct(&sb, s, ns)
			sb.WriteString("\n")
		}
	}

	// Write file
	dir := filepath.Join(g.baseDir, strings.ToLower(ns.Name))
	if err := os.MkdirAll(dir, 0755); err != nil {
		return err
	}

	filePath := filepath.Join(dir, "types.ts")
	return os.WriteFile(filePath, []byte(sb.String()), 0644)
}

func (g *Generator) generateClientFile(ns *ast.Namespace, baseName string) error {
	var sb strings.Builder

	g.printHeader(&sb)

	// Imports
	sb.WriteString("import { TarsInputStream, TarsOutputStream } from '@aspect/tars-stream';\n")
	sb.WriteString("import { ServantProxy, TarsContext, InvokeOptions } from '@aspect/tars-rpc';\n")

	// Import types
	var typeImports []string
	typeSet := make(map[string]bool)
	for _, iface := range ns.Interfaces {
		for _, op := range iface.Operations {
			g.collectTypeImports(op.ReturnType, typeSet)
			for _, p := range op.Params {
				g.collectTypeImports(p.Type, typeSet)
			}
		}
	}
	for t := range typeSet {
		typeImports = append(typeImports, t)
	}
	if len(typeImports) > 0 {
		sb.WriteString(fmt.Sprintf("import { %s } from './types';\n", strings.Join(typeImports, ", ")))
	}
	sb.WriteString("\n")

	// Generate client proxies
	for _, iface := range ns.Interfaces {
		g.generateClientProxy(&sb, iface, ns)
		sb.WriteString("\n")
	}

	// Write file
	dir := filepath.Join(g.baseDir, strings.ToLower(ns.Name))
	filePath := filepath.Join(dir, "client.ts")
	return os.WriteFile(filePath, []byte(sb.String()), 0644)
}

func (g *Generator) generateServerFile(ns *ast.Namespace, baseName string) error {
	var sb strings.Builder

	g.printHeader(&sb)

	// Imports
	sb.WriteString("import { TarsInputStream, TarsOutputStream } from '@aspect/tars-stream';\n")
	sb.WriteString("import { TarsContext } from '@aspect/tars-rpc';\n")

	// Import types
	var typeImports []string
	typeSet := make(map[string]bool)
	for _, iface := range ns.Interfaces {
		for _, op := range iface.Operations {
			g.collectTypeImports(op.ReturnType, typeSet)
			for _, p := range op.Params {
				g.collectTypeImports(p.Type, typeSet)
			}
		}
	}
	for t := range typeSet {
		typeImports = append(typeImports, t)
	}
	if len(typeImports) > 0 {
		sb.WriteString(fmt.Sprintf("import { %s } from './types';\n", strings.Join(typeImports, ", ")))
	}
	sb.WriteString("\n")

	// Generate server interfaces
	for _, iface := range ns.Interfaces {
		g.generateServerInterface(&sb, iface, ns)
		sb.WriteString("\n")

		// Generate servant implementation base class
		g.generateServantImpl(&sb, iface, ns)
		sb.WriteString("\n")
	}

	// Write file
	dir := filepath.Join(g.baseDir, strings.ToLower(ns.Name))
	filePath := filepath.Join(dir, "server.ts")
	return os.WriteFile(filePath, []byte(sb.String()), 0644)
}

func (g *Generator) generateIndexFile(ns *ast.Namespace, baseName string) error {
	var sb strings.Builder

	g.printHeader(&sb)

	sb.WriteString("// Re-export all types\n")
	sb.WriteString("export * from './types';\n")

	if len(ns.Interfaces) > 0 {
		sb.WriteString("\n// Re-export client proxies\n")
		sb.WriteString("export * from './client';\n")
		sb.WriteString("\n// Re-export server interfaces\n")
		sb.WriteString("export * from './server';\n")
	}

	// Write file
	dir := filepath.Join(g.baseDir, strings.ToLower(ns.Name))
	filePath := filepath.Join(dir, "index.ts")
	return os.WriteFile(filePath, []byte(sb.String()), 0644)
}

func (g *Generator) printHeader(sb *strings.Builder) {
	sb.WriteString("/**\n")
	sb.WriteString(" * This file was generated by tarsc - Tars IDL Compiler\n")
	sb.WriteString(" * DO NOT EDIT!\n")
	sb.WriteString(" *\n")
	sb.WriteString(" * @generated\n")
	sb.WriteString(" */\n\n")
}

func (g *Generator) generateEnum(sb *strings.Builder, e *ast.Enum) {
	// Generate const enum for better performance
	sb.WriteString(fmt.Sprintf("export const enum %s {\n", e.Name))
	for _, m := range e.Members {
		sb.WriteString(fmt.Sprintf("    %s = %d,\n", m.Name, m.Value))
	}
	sb.WriteString("}\n\n")

	// Generate reverse mapping object
	sb.WriteString(fmt.Sprintf("export const %sNames: Record<number, string> = {\n", e.Name))
	for _, m := range e.Members {
		sb.WriteString(fmt.Sprintf("    [%s.%s]: '%s',\n", e.Name, m.Name, m.Name))
	}
	sb.WriteString("};\n")
}

func (g *Generator) generateStruct(sb *strings.Builder, s *ast.Struct, ns *ast.Namespace) {
	// Generate interface for the struct (for type checking)
	sb.WriteString(fmt.Sprintf("export interface I%s {\n", s.Name))
	for _, f := range s.Fields {
		optional := ""
		if !f.IsRequired {
			optional = "?"
		}
		sb.WriteString(fmt.Sprintf("    %s%s: %s;\n", f.Name, optional, g.toTSType(f.Type)))
	}
	sb.WriteString("}\n\n")

	// Generate class for serialization
	sb.WriteString(fmt.Sprintf("export class %s implements I%s {\n", s.Name, s.Name))

	// Static cache for complex types (used in readFrom)
	for _, f := range s.Fields {
		if g.needsCache(f.Type) {
			sb.WriteString(fmt.Sprintf("    private static readonly _cache_%s = %s;\n", f.Name, g.getCacheInit(f.Type)))
		}
	}
	if g.hasComplexFields(s.Fields) {
		sb.WriteString("\n")
	}

	// Fields with default values
	for _, f := range s.Fields {
		defaultVal := g.getDefaultValue(f)
		sb.WriteString(fmt.Sprintf("    public %s: %s = %s;\n", f.Name, g.toTSType(f.Type), defaultVal))
	}
	sb.WriteString("\n")

	// Constructor
	sb.WriteString(fmt.Sprintf("    constructor(data?: Partial<I%s>) {\n", s.Name))
	sb.WriteString("        if (data) {\n")
	sb.WriteString("            Object.assign(this, data);\n")
	sb.WriteString("        }\n")
	sb.WriteString("    }\n\n")

	// Static create method
	sb.WriteString(fmt.Sprintf("    static create(data?: Partial<I%s>): %s {\n", s.Name, s.Name))
	sb.WriteString(fmt.Sprintf("        return new %s(data);\n", s.Name))
	sb.WriteString("    }\n\n")

	// writeTo method
	sb.WriteString("    writeTo(os: TarsOutputStream): void {\n")
	for _, f := range s.Fields {
		if !f.IsRequired {
			sb.WriteString(fmt.Sprintf("        if (this.%s !== undefined && this.%s !== null) {\n", f.Name, f.Name))
			sb.WriteString(fmt.Sprintf("            os.%s(%d, this.%s);\n", g.getWriteMethod(f.Type), f.Tag, f.Name))
			sb.WriteString("        }\n")
		} else {
			sb.WriteString(fmt.Sprintf("        os.%s(%d, this.%s);\n", g.getWriteMethod(f.Type), f.Tag, f.Name))
		}
	}
	sb.WriteString("    }\n\n")

	// readFrom method
	sb.WriteString("    readFrom(is: TarsInputStream): void {\n")
	for _, f := range s.Fields {
		if g.needsCache(f.Type) {
			sb.WriteString(fmt.Sprintf("        this.%s = is.%s(%d, %t, %s._cache_%s);\n",
				f.Name, g.getReadMethod(f.Type), f.Tag, f.IsRequired, s.Name, f.Name))
		} else {
			sb.WriteString(fmt.Sprintf("        this.%s = is.%s(%d, %t, this.%s);\n",
				f.Name, g.getReadMethod(f.Type), f.Tag, f.IsRequired, f.Name))
		}
	}
	sb.WriteString("    }\n\n")

	// toJSON method
	sb.WriteString(fmt.Sprintf("    toJSON(): I%s {\n", s.Name))
	sb.WriteString("        return {\n")
	for i, f := range s.Fields {
		comma := ","
		if i == len(s.Fields)-1 {
			comma = ""
		}
		sb.WriteString(fmt.Sprintf("            %s: this.%s%s\n", f.Name, f.Name, comma))
	}
	sb.WriteString("        };\n")
	sb.WriteString("    }\n\n")

	// toString method
	sb.WriteString("    toString(): string {\n")
	sb.WriteString("        return JSON.stringify(this.toJSON());\n")
	sb.WriteString("    }\n")

	sb.WriteString("}\n")
}

func (g *Generator) generateClientProxy(sb *strings.Builder, iface *ast.Interface, ns *ast.Namespace) {
	proxyName := iface.Name + "Proxy"

	// Generate callback types for each operation
	for _, op := range iface.Operations {
		g.generateCallbackType(sb, op, iface.Name)
	}

	// Generate proxy class
	sb.WriteString(fmt.Sprintf("export class %s extends ServantProxy {\n", proxyName))

	// Servant name
	sb.WriteString(fmt.Sprintf("    static readonly servantName = '%s.%s';\n\n", ns.Name, iface.Name))

	for _, op := range iface.Operations {
		g.generateProxyMethod(sb, op, iface.Name)
		sb.WriteString("\n")
	}

	sb.WriteString("}\n")
}

func (g *Generator) generateCallbackType(sb *strings.Builder, op *ast.Operation, ifaceName string) {
	callbackName := fmt.Sprintf("%s%sCallback", ifaceName, firstUpper(op.Name))

	// Collect out params
	var outParams []string
	for _, p := range op.Params {
		if p.IsOut {
			outParams = append(outParams, fmt.Sprintf("%s: %s", p.Name, g.toTSType(p.Type)))
		}
	}

	returnType := g.toTSType(op.ReturnType)
	if returnType != "void" {
		outParams = append([]string{fmt.Sprintf("result: %s", returnType)}, outParams...)
	}

	if len(outParams) == 0 {
		sb.WriteString(fmt.Sprintf("export type %s = () => void;\n\n", callbackName))
	} else {
		sb.WriteString(fmt.Sprintf("export type %s = (%s) => void;\n\n", callbackName, strings.Join(outParams, ", ")))
	}
}

func (g *Generator) generateProxyMethod(sb *strings.Builder, op *ast.Operation, ifaceName string) {
	// Collect input params
	var inputParams []string
	for _, p := range op.Params {
		if !p.IsOut {
			inputParams = append(inputParams, fmt.Sprintf("%s: %s", p.Name, g.toTSType(p.Type)))
		}
	}

	// Collect out params for return type
	var outTypes []string
	for _, p := range op.Params {
		if p.IsOut {
			outTypes = append(outTypes, fmt.Sprintf("%s: %s", p.Name, g.toTSType(p.Type)))
		}
	}

	returnType := g.toTSType(op.ReturnType)
	if returnType != "void" {
		outTypes = append([]string{fmt.Sprintf("result: %s", returnType)}, outTypes...)
	}

	// Determine return Promise type
	promiseType := "void"
	if len(outTypes) == 1 {
		promiseType = strings.Split(outTypes[0], ": ")[1]
	} else if len(outTypes) > 1 {
		promiseType = fmt.Sprintf("{ %s }", strings.Join(outTypes, "; "))
	}

	// Add options parameter
	allParams := append(inputParams, "options?: InvokeOptions")

	// Method signature
	sb.WriteString(fmt.Sprintf("    async %s(%s): Promise<%s> {\n", op.Name, strings.Join(allParams, ", "), promiseType))

	// Create output stream
	sb.WriteString("        const os = new TarsOutputStream();\n")

	// Write input parameters
	tagNum := 1
	for _, p := range op.Params {
		if !p.IsOut {
			sb.WriteString(fmt.Sprintf("        os.%s(%d, %s);\n", g.getWriteMethod(p.Type), tagNum, p.Name))
		}
		tagNum++
	}

	// Invoke RPC
	sb.WriteString(fmt.Sprintf("        const response = await this.invoke('%s', os.getBuffer(), options);\n", op.Name))

	// Parse response
	sb.WriteString("        const is = new TarsInputStream(response.buffer);\n")

	// Read return value and out parameters
	var readStatements []string
	if returnType != "void" {
		readStatements = append(readStatements, fmt.Sprintf("const result = is.%s(0, true, %s)", g.getReadMethod(op.ReturnType), g.getDefaultValue(&ast.Field{Type: op.ReturnType})))
	}

	tagNum = 1
	for _, p := range op.Params {
		if p.IsOut {
			readStatements = append(readStatements, fmt.Sprintf("const %s = is.%s(%d, true, %s)", p.Name, g.getReadMethod(p.Type), tagNum, g.getDefaultValue(&ast.Field{Type: p.Type})))
		}
		tagNum++
	}

	for _, stmt := range readStatements {
		sb.WriteString(fmt.Sprintf("        %s;\n", stmt))
	}

	// Return
	if len(outTypes) == 0 {
		// No return
	} else if len(outTypes) == 1 {
		varName := strings.Split(outTypes[0], ":")[0]
		sb.WriteString(fmt.Sprintf("        return %s;\n", strings.TrimSpace(varName)))
	} else {
		var returnVars []string
		for _, ot := range outTypes {
			varName := strings.Split(ot, ":")[0]
			returnVars = append(returnVars, strings.TrimSpace(varName))
		}
		sb.WriteString(fmt.Sprintf("        return { %s };\n", strings.Join(returnVars, ", ")))
	}

	sb.WriteString("    }\n")
}

func (g *Generator) generateServerInterface(sb *strings.Builder, iface *ast.Interface, ns *ast.Namespace) {
	interfaceName := "I" + iface.Name + "Servant"

	sb.WriteString(fmt.Sprintf("export interface %s {\n", interfaceName))

	for _, op := range iface.Operations {
		// Collect all params
		var allParams []string
		for _, p := range op.Params {
			paramType := g.toTSType(p.Type)
			if p.IsOut {
				paramType = fmt.Sprintf("{ value: %s }", paramType)
			}
			allParams = append(allParams, fmt.Sprintf("%s: %s", p.Name, paramType))
		}
		allParams = append(allParams, "ctx: TarsContext")

		returnType := g.toTSType(op.ReturnType)
		if returnType == "void" {
			returnType = "Promise<void>"
		} else {
			returnType = fmt.Sprintf("Promise<%s>", returnType)
		}

		sb.WriteString(fmt.Sprintf("    %s(%s): %s;\n", op.Name, strings.Join(allParams, ", "), returnType))
	}

	sb.WriteString("}\n")
}

func (g *Generator) generateServantImpl(sb *strings.Builder, iface *ast.Interface, ns *ast.Namespace) {
	className := iface.Name + "Servant"
	interfaceName := "I" + iface.Name + "Servant"

	sb.WriteString(fmt.Sprintf("export abstract class %s implements %s {\n", className, interfaceName))

	// Servant name
	sb.WriteString(fmt.Sprintf("    static readonly servantName = '%s.%s';\n\n", ns.Name, iface.Name))

	// Dispatch method
	sb.WriteString("    async dispatch(funcName: string, is: TarsInputStream, ctx: TarsContext): Promise<TarsOutputStream> {\n")
	sb.WriteString("        const os = new TarsOutputStream();\n")
	sb.WriteString("        switch (funcName) {\n")

	for _, op := range iface.Operations {
		sb.WriteString(fmt.Sprintf("            case '%s': {\n", op.Name))

		// Read input parameters
		tagNum := 1
		for _, p := range op.Params {
			if p.IsOut {
				sb.WriteString(fmt.Sprintf("                const %s = { value: %s };\n", p.Name, g.getDefaultValue(&ast.Field{Type: p.Type})))
			} else {
				sb.WriteString(fmt.Sprintf("                const %s = is.%s(%d, true, %s);\n",
					p.Name, g.getReadMethod(p.Type), tagNum, g.getDefaultValue(&ast.Field{Type: p.Type})))
			}
			tagNum++
		}

		// Call implementation
		var callParams []string
		for _, p := range op.Params {
			callParams = append(callParams, p.Name)
		}
		callParams = append(callParams, "ctx")

		returnType := g.toTSType(op.ReturnType)
		if returnType != "void" {
			sb.WriteString(fmt.Sprintf("                const result = await this.%s(%s);\n", op.Name, strings.Join(callParams, ", ")))
			sb.WriteString(fmt.Sprintf("                os.%s(0, result);\n", g.getWriteMethod(op.ReturnType)))
		} else {
			sb.WriteString(fmt.Sprintf("                await this.%s(%s);\n", op.Name, strings.Join(callParams, ", ")))
		}

		// Write out parameters
		tagNum = 1
		for _, p := range op.Params {
			if p.IsOut {
				sb.WriteString(fmt.Sprintf("                os.%s(%d, %s.value);\n", g.getWriteMethod(p.Type), tagNum, p.Name))
			}
			tagNum++
		}

		sb.WriteString("                break;\n")
		sb.WriteString("            }\n")
	}

	sb.WriteString("            default:\n")
	sb.WriteString("                throw new Error(`Unknown function: ${funcName}`);\n")
	sb.WriteString("        }\n")
	sb.WriteString("        return os;\n")
	sb.WriteString("    }\n\n")

	// Abstract methods
	for _, op := range iface.Operations {
		var allParams []string
		for _, p := range op.Params {
			paramType := g.toTSType(p.Type)
			if p.IsOut {
				paramType = fmt.Sprintf("{ value: %s }", paramType)
			}
			allParams = append(allParams, fmt.Sprintf("%s: %s", p.Name, paramType))
		}
		allParams = append(allParams, "ctx: TarsContext")

		returnType := g.toTSType(op.ReturnType)
		if returnType == "void" {
			returnType = "Promise<void>"
		} else {
			returnType = fmt.Sprintf("Promise<%s>", returnType)
		}

		sb.WriteString(fmt.Sprintf("    abstract %s(%s): %s;\n", op.Name, strings.Join(allParams, ", "), returnType))
	}

	sb.WriteString("}\n")
}

func (g *Generator) toTSType(t ast.Type) string {
	if t == nil {
		return "void"
	}

	switch v := t.(type) {
	case *ast.BuiltinType:
		switch v.Kind {
		case ast.KindVoid:
			return "void"
		case ast.KindBool:
			return "boolean"
		case ast.KindByte, ast.KindShort, ast.KindInt, ast.KindLong, ast.KindFloat, ast.KindDouble:
			return "number"
		case ast.KindString:
			return "string"
		}
	case *ast.VectorType:
		if bt, ok := v.ElementType.(*ast.BuiltinType); ok && bt.Kind == ast.KindByte {
			return "Uint8Array"
		}
		return fmt.Sprintf("Array<%s>", g.toTSType(v.ElementType))
	case *ast.MapType:
		return fmt.Sprintf("Map<%s, %s>", g.toTSType(v.KeyType), g.toTSType(v.ValueType))
	case *ast.CustomType:
		return v.TypeName_
	}
	return "unknown"
}

func (g *Generator) getDefaultValue(f *ast.Field) string {
	if f.HasDefault && f.DefaultValue != "" {
		return f.DefaultValue
	}

	return g.getTypeDefault(f.Type)
}

func (g *Generator) getTypeDefault(t ast.Type) string {
	if t == nil {
		return "undefined"
	}

	switch v := t.(type) {
	case *ast.BuiltinType:
		switch v.Kind {
		case ast.KindBool:
			return "false"
		case ast.KindString:
			return "''"
		case ast.KindByte, ast.KindShort, ast.KindInt, ast.KindLong:
			return "0"
		case ast.KindFloat, ast.KindDouble:
			return "0.0"
		}
	case *ast.VectorType:
		if bt, ok := v.ElementType.(*ast.BuiltinType); ok && bt.Kind == ast.KindByte {
			return "new Uint8Array()"
		}
		return "[]"
	case *ast.MapType:
		return "new Map()"
	case *ast.CustomType:
		if g.enums[v.Name] || g.enums[v.TypeName_] {
			return "0"
		}
		return fmt.Sprintf("new %s()", v.TypeName_)
	}
	return "undefined"
}

func (g *Generator) getWriteMethod(t ast.Type) string {
	switch v := t.(type) {
	case *ast.BuiltinType:
		switch v.Kind {
		case ast.KindBool:
			return "writeBoolean"
		case ast.KindByte:
			return "writeInt8"
		case ast.KindShort:
			if v.IsUnsigned {
				return "writeUInt16"
			}
			return "writeInt16"
		case ast.KindInt:
			if v.IsUnsigned {
				return "writeUInt32"
			}
			return "writeInt32"
		case ast.KindLong:
			if v.IsUnsigned {
				return "writeUInt64"
			}
			return "writeInt64"
		case ast.KindFloat:
			return "writeFloat"
		case ast.KindDouble:
			return "writeDouble"
		case ast.KindString:
			return "writeString"
		}
	case *ast.VectorType:
		if bt, ok := v.ElementType.(*ast.BuiltinType); ok && bt.Kind == ast.KindByte {
			return "writeBytes"
		}
		return "writeVector"
	case *ast.MapType:
		return "writeMap"
	case *ast.CustomType:
		if g.enums[v.Name] || g.enums[v.TypeName_] {
			return "writeInt32"
		}
		return "writeStruct"
	}
	return "writeInt32"
}

func (g *Generator) getReadMethod(t ast.Type) string {
	switch v := t.(type) {
	case *ast.BuiltinType:
		switch v.Kind {
		case ast.KindBool:
			return "readBoolean"
		case ast.KindByte:
			return "readInt8"
		case ast.KindShort:
			if v.IsUnsigned {
				return "readUInt16"
			}
			return "readInt16"
		case ast.KindInt:
			if v.IsUnsigned {
				return "readUInt32"
			}
			return "readInt32"
		case ast.KindLong:
			if v.IsUnsigned {
				return "readUInt64"
			}
			return "readInt64"
		case ast.KindFloat:
			return "readFloat"
		case ast.KindDouble:
			return "readDouble"
		case ast.KindString:
			return "readString"
		}
	case *ast.VectorType:
		if bt, ok := v.ElementType.(*ast.BuiltinType); ok && bt.Kind == ast.KindByte {
			return "readBytes"
		}
		return "readVector"
	case *ast.MapType:
		return "readMap"
	case *ast.CustomType:
		if g.enums[v.Name] || g.enums[v.TypeName_] {
			return "readInt32"
		}
		return "readStruct"
	}
	return "readInt32"
}

func (g *Generator) needsCache(t ast.Type) bool {
	switch v := t.(type) {
	case *ast.VectorType:
		return true
	case *ast.MapType:
		return true
	case *ast.CustomType:
		return !g.enums[v.Name] && !g.enums[v.TypeName_]
	}
	return false
}

func (g *Generator) getCacheInit(t ast.Type) string {
	switch v := t.(type) {
	case *ast.VectorType:
		if bt, ok := v.ElementType.(*ast.BuiltinType); ok && bt.Kind == ast.KindByte {
			return "new Uint8Array()"
		}
		return fmt.Sprintf("[%s]", g.getTypeDefault(v.ElementType))
	case *ast.MapType:
		return fmt.Sprintf("new Map([[%s, %s]])", g.getTypeDefault(v.KeyType), g.getTypeDefault(v.ValueType))
	case *ast.CustomType:
		return fmt.Sprintf("new %s()", v.TypeName_)
	}
	return "undefined"
}

func (g *Generator) hasComplexFields(fields []*ast.Field) bool {
	for _, f := range fields {
		if g.needsCache(f.Type) {
			return true
		}
	}
	return false
}

func (g *Generator) collectTypeImports(t ast.Type, imports map[string]bool) {
	if t == nil {
		return
	}

	switch v := t.(type) {
	case *ast.VectorType:
		g.collectTypeImports(v.ElementType, imports)
	case *ast.MapType:
		g.collectTypeImports(v.KeyType, imports)
		g.collectTypeImports(v.ValueType, imports)
	case *ast.CustomType:
		imports[v.TypeName_] = true
	}
}

func firstUpper(s string) string {
	if len(s) == 0 {
		return s
	}
	return strings.ToUpper(s[:1]) + s[1:]
}
