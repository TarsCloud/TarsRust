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

package main

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/ast"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/generator/cpp"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/generator/csharp"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/generator/golang"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/generator/java"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/generator/node"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/generator/objc"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/generator/php"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/generator/python"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/generator/rust"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/generator/swift"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/generator/typescript"
	"github.com/TarsCloud/TarsCpp/tools/tarsc-go/pkg/parser"
)

const (
	VERSION   = "1.0.0"
	BuildDate = "2026-01"
)

// ANSI 颜色码
const (
	colorReset  = "\033[0m"
	colorRed    = "\033[31m"
	colorGreen  = "\033[32m"
	colorYellow = "\033[33m"
	colorBlue   = "\033[34m"
	colorCyan   = "\033[36m"
	colorBold   = "\033[1m"
)

// outputFlags 用于收集多个 -o/--out 参数
type outputFlags []string

func (o *outputFlags) String() string {
	return strings.Join(*o, ", ")
}

func (o *outputFlags) Set(value string) error {
	*o = append(*o, value)
	return nil
}

// 语言信息
type langInfo struct {
	name        string // 命令行参数名
	displayName string // 显示名称
	description string // 描述
	fileExt     string // 生成文件扩展名
	aliases     []string // 别名
}

// 支持的语言详细信息
var languages = []langInfo{
	{name: "go", displayName: "Go", description: "Go 语言代码", fileExt: ".go", aliases: []string{"golang"}},
	{name: "java", displayName: "Java", description: "Java 代码 (Prx/Servant/Callback)", fileExt: ".java"},
	{name: "cpp", displayName: "C++", description: "C++ 代码", fileExt: ".h/.cpp", aliases: []string{"c++"}},
	{name: "python", displayName: "Python", description: "Python 代码", fileExt: ".py", aliases: []string{"py"}},
	{name: "typescript", displayName: "TypeScript", description: "TypeScript 代码 (类型/客户端/服务端)", fileExt: ".ts", aliases: []string{"ts"}},
	{name: "swift", displayName: "Swift", description: "Swift 代码 (struct/protocol/async)", fileExt: ".swift"},
	{name: "rust", displayName: "Rust", description: "Rust 代码", fileExt: ".rs"},
	{name: "csharp", displayName: "C#", description: "C# 代码", fileExt: ".cs", aliases: []string{"cs"}},
	{name: "node", displayName: "Node.js", description: "Node.js 代码", fileExt: ".js", aliases: []string{"nodejs", "js"}},
	{name: "php", displayName: "PHP", description: "PHP 代码", fileExt: ".php"},
	{name: "objc", displayName: "Objective-C", description: "Objective-C 代码", fileExt: ".h/.m", aliases: []string{"oc"}},
}

// 语言显示名称映射
var langDisplayNames = make(map[string]string)

// 语言别名映射 (别名 -> 标准名)
var langAliases = make(map[string]string)

// 支持的语言列表 (用于验证)
var supportedLangs []string

func init() {
	for _, lang := range languages {
		langDisplayNames[lang.name] = lang.displayName
		supportedLangs = append(supportedLangs, lang.name)
		for _, alias := range lang.aliases {
			langAliases[alias] = lang.name
			langDisplayNames[alias] = lang.displayName
			supportedLangs = append(supportedLangs, alias)
		}
	}
}

var (
	outputs     outputFlags
	includePath = flag.String("I", "", "")
	showVersion = flag.Bool("version", false, "")
	showHelp    = flag.Bool("help", false, "")
	showLangs   = flag.Bool("list-langs", false, "")
	verbose     = flag.Bool("v", false, "")
	noColor     = flag.Bool("no-color", false, "")
)

// langOutputs 存储每种语言的输出目录
var langOutputs = make(map[string]string)

// 统计信息
type stats struct {
	structs    int
	enums      int
	consts     int
	interfaces int
}

func main() {
	// 注册 -o/--out 参数（可多次使用）
	flag.Var(&outputs, "o", "")
	flag.Var(&outputs, "out", "")

	flag.Usage = printUsage
	flag.Parse()

	// 禁用颜色
	if *noColor || os.Getenv("NO_COLOR") != "" {
		disableColors()
	}

	// 显示帮助
	if *showHelp {
		printUsage()
		os.Exit(0)
	}

	// 显示版本
	if *showVersion {
		printVersion()
		os.Exit(0)
	}

	// 显示支持的语言
	if *showLangs {
		printLanguages()
		os.Exit(0)
	}

	// 检查输入文件
	if flag.NArg() == 0 {
		printError("未指定输入文件")
		fmt.Fprintln(os.Stderr)
		fmt.Fprintln(os.Stderr, "用法: tarsc -o <语言>:<输出目录> <tars文件>...")
		fmt.Fprintln(os.Stderr, "运行 'tarsc --help' 查看详细帮助")
		os.Exit(1)
	}

	// 解析 -o 参数
	if err := parseOutputFlags(); err != nil {
		printError(err.Error())
		os.Exit(1)
	}

	// 检查至少指定了一个输出
	if len(langOutputs) == 0 {
		printError("未指定输出语言")
		fmt.Fprintln(os.Stderr)
		fmt.Fprintln(os.Stderr, "请使用 -o 参数指定输出语言和目录，例如:")
		fmt.Fprintln(os.Stderr, "  tarsc -o go:./output message.tars")
		fmt.Fprintln(os.Stderr)
		fmt.Fprintln(os.Stderr, "运行 'tarsc --list-langs' 查看支持的语言列表")
		os.Exit(1)
	}

	// Parse include paths
	var includePaths []string
	if *includePath != "" {
		includePaths = strings.Split(*includePath, ";")
	}

	// 打印开始信息
	if *verbose {
		printInfo("Tars IDL Compiler v%s", VERSION)
		printInfo("输入文件: %d 个", flag.NArg())
		printInfo("目标语言: %s", formatTargetLangs())
		fmt.Println()
	}

	// 统计
	totalFiles := 0
	totalGenerated := 0

	// Process each input file
	for _, inputFile := range flag.Args() {
		generated, err := processFile(inputFile, includePaths)
		if err != nil {
			printError("处理文件 '%s' 失败: %v", inputFile, err)
			os.Exit(1)
		}
		totalFiles++
		totalGenerated += generated
	}

	// 打印完成信息
	fmt.Println()
	printSuccess("代码生成完成!")
	printSuccess("  处理文件: %d 个", totalFiles)
	printSuccess("  生成文件: %d 个", totalGenerated)
}

// printUsage 打印使用帮助
func printUsage() {
	w := os.Stderr

	fmt.Fprintf(w, "%s%starsc%s - Tars IDL 编译器\n", colorBold, colorCyan, colorReset)
	fmt.Fprintf(w, "将 Tars IDL 文件编译为多种编程语言的代码\n\n")

	fmt.Fprintf(w, "%s用法:%s\n", colorBold, colorReset)
	fmt.Fprintf(w, "  tarsc [选项] <tars文件>...\n\n")

	fmt.Fprintf(w, "%s选项:%s\n", colorBold, colorReset)
	fmt.Fprintf(w, "  -o <语言>:<目录>     指定输出语言和目录 (可多次使用)\n")
	fmt.Fprintf(w, "  --out=<语言>:<目录>  同 -o\n")
	fmt.Fprintf(w, "  -I <路径>            指定 include 搜索路径 (多个路径用 ; 分隔)\n")
	fmt.Fprintf(w, "  -v                   显示详细处理信息\n")
	fmt.Fprintf(w, "  --no-color           禁用彩色输出\n")
	fmt.Fprintf(w, "  --list-langs         显示支持的语言列表\n")
	fmt.Fprintf(w, "  --version            显示版本信息\n")
	fmt.Fprintf(w, "  --help               显示此帮助信息\n\n")

	fmt.Fprintf(w, "%s支持的语言:%s\n", colorBold, colorReset)
	for _, lang := range languages {
		aliases := ""
		if len(lang.aliases) > 0 {
			aliases = fmt.Sprintf(" (别名: %s)", strings.Join(lang.aliases, ", "))
		}
		fmt.Fprintf(w, "  %-12s %s%s\n", lang.name, lang.description, aliases)
	}
	fmt.Fprintln(w)

	fmt.Fprintf(w, "%s示例:%s\n", colorBold, colorReset)
	fmt.Fprintf(w, "  %s# 生成 Go 代码%s\n", colorYellow, colorReset)
	fmt.Fprintf(w, "  tarsc -o go:./gen/go message.tars\n\n")

	fmt.Fprintf(w, "  %s# 同时生成多种语言%s\n", colorYellow, colorReset)
	fmt.Fprintf(w, "  tarsc -o go:./gen/go -o java:./gen/java -o ts:./gen/ts message.tars\n\n")

	fmt.Fprintf(w, "  %s# 指定 include 路径%s\n", colorYellow, colorReset)
	fmt.Fprintf(w, "  tarsc -I /path/to/includes -o go:./gen message.tars\n\n")

	fmt.Fprintf(w, "  %s# 处理多个文件%s\n", colorYellow, colorReset)
	fmt.Fprintf(w, "  tarsc -o java:./gen *.tars\n\n")

	fmt.Fprintf(w, "  %s# 详细模式%s\n", colorYellow, colorReset)
	fmt.Fprintf(w, "  tarsc -v -o go:./gen message.tars\n\n")

	fmt.Fprintf(w, "%s生成内容:%s\n", colorBold, colorReset)
	fmt.Fprintf(w, "  • struct/enum/const   类型定义\n")
	fmt.Fprintf(w, "  • Prx (客户端代理)    用于调用远程服务\n")
	fmt.Fprintf(w, "  • Servant (服务端)    用于实现服务接口\n")
	fmt.Fprintf(w, "  • Callback (回调)     用于异步调用\n\n")

	fmt.Fprintf(w, "%s更多信息:%s\n", colorBold, colorReset)
	fmt.Fprintf(w, "  项目地址: https://github.com/TarsCloud/TarsCpp\n")
}

// printVersion 打印版本信息
func printVersion() {
	fmt.Printf("%s%starsc%s - Tars IDL Compiler\n", colorBold, colorCyan, colorReset)
	fmt.Printf("版本: %s\n", VERSION)
	fmt.Printf("构建: %s\n", BuildDate)
	fmt.Printf("支持语言: %d 种\n", len(languages))
	fmt.Println()
	fmt.Println("Copyright (C) 2016 THL A29 Limited, a Tencent company.")
	fmt.Println("Licensed under the BSD 3-Clause License")
}

// printLanguages 打印支持的语言列表
func printLanguages() {
	fmt.Printf("%s%s支持的语言列表:%s\n\n", colorBold, colorCyan, colorReset)

	// 计算最大宽度
	maxNameLen := 0
	for _, lang := range languages {
		if len(lang.name) > maxNameLen {
			maxNameLen = len(lang.name)
		}
	}

	fmt.Printf("  %-*s  %-12s  %-8s  %s\n", maxNameLen, "语言", "显示名", "扩展名", "说明")
	fmt.Printf("  %s\n", strings.Repeat("-", 60))

	for _, lang := range languages {
		aliases := ""
		if len(lang.aliases) > 0 {
			aliases = fmt.Sprintf("(别名: %s)", strings.Join(lang.aliases, ", "))
		}
		fmt.Printf("  %-*s  %-12s  %-8s  %s %s\n",
			maxNameLen, lang.name, lang.displayName, lang.fileExt, lang.description, aliases)
	}

	fmt.Println()
	fmt.Println("使用示例:")
	fmt.Println("  tarsc -o go:./output message.tars")
	fmt.Println("  tarsc -o typescript:./output message.tars")
	fmt.Println("  tarsc -o ts:./output message.tars  # 使用别名")
}

// parseOutputFlags 解析 -o 参数，格式为 LANG:DIR
func parseOutputFlags() error {
	if len(outputs) == 0 {
		return nil
	}

	for _, out := range outputs {
		parts := strings.SplitN(out, ":", 2)
		if len(parts) != 2 {
			return fmt.Errorf("输出格式错误: '%s'\n"+
				"  正确格式: <语言>:<目录>\n"+
				"  例如: go:./output 或 java:./gen/java", out)
		}

		lang := strings.ToLower(strings.TrimSpace(parts[0]))
		dir := strings.TrimSpace(parts[1])

		if lang == "" {
			return fmt.Errorf("语言名称不能为空: '%s'", out)
		}

		if dir == "" {
			return fmt.Errorf("输出目录不能为空: '%s'\n"+
				"  例如: %s:./output", out, lang)
		}

		// 检查别名
		if canonical, ok := langAliases[lang]; ok {
			lang = canonical
		}

		// 验证语言是否支持
		if !isLangSupported(lang) {
			// 尝试模糊匹配建议
			suggestion := suggestLanguage(lang)
			errMsg := fmt.Sprintf("不支持的语言: '%s'", lang)
			if suggestion != "" {
				errMsg += fmt.Sprintf("\n  您是否想使用: %s?", suggestion)
			}
			errMsg += "\n  运行 'tarsc --list-langs' 查看支持的语言"
			return fmt.Errorf(errMsg)
		}

		langOutputs[lang] = dir
	}
	return nil
}

// suggestLanguage 尝试建议正确的语言名
func suggestLanguage(input string) string {
	input = strings.ToLower(input)

	// 常见拼写错误和别名 (优先检查)
	corrections := map[string]string{
		"golang":      "go",
		"javascript":  "node",
		"js":          "node",
		"c#":          "csharp",
		"c++":         "cpp",
		"objective-c": "objc",
		"objectivec":  "objc",
		"py":          "python",
		"ts":          "typescript",
		"typescript":  "typescript",
	}

	if suggestion, ok := corrections[input]; ok {
		return suggestion
	}

	// 模糊匹配
	for _, lang := range languages {
		if strings.Contains(lang.name, input) || strings.Contains(input, lang.name) {
			return lang.name
		}
		for _, alias := range lang.aliases {
			if strings.Contains(alias, input) || strings.Contains(input, alias) {
				return lang.name
			}
		}
	}

	return ""
}

// isLangSupported 检查语言是否支持
func isLangSupported(lang string) bool {
	for _, l := range languages {
		if l.name == lang {
			return true
		}
	}
	return false
}

// formatTargetLangs 格式化目标语言列表
func formatTargetLangs() string {
	var langs []string
	for lang := range langOutputs {
		displayName := langDisplayNames[lang]
		if displayName == "" {
			displayName = lang
		}
		langs = append(langs, displayName)
	}
	sort.Strings(langs)
	return strings.Join(langs, ", ")
}

func processFile(inputFile string, includePaths []string) (int, error) {
	// 检查文件是否存在
	if _, err := os.Stat(inputFile); os.IsNotExist(err) {
		return 0, fmt.Errorf("文件不存在: %s", inputFile)
	}

	// 检查文件扩展名
	ext := strings.ToLower(filepath.Ext(inputFile))
	if ext != ".tars" {
		printWarning("文件 '%s' 不是 .tars 文件，尝试继续处理...", inputFile)
	}

	// 打印处理信息
	printStep("处理文件: %s", inputFile)

	// Read the input file
	content, err := os.ReadFile(inputFile)
	if err != nil {
		return 0, fmt.Errorf("读取文件失败: %w", err)
	}

	// Add the directory of the input file to include paths
	inputDir := filepath.Dir(inputFile)
	allIncludePaths := append([]string{inputDir}, includePaths...)

	// Parse the file
	if *verbose {
		printDetail("解析 IDL 文件...")
	}

	p := parser.NewParser(string(content), inputFile, allIncludePaths)
	document, err := p.Parse()
	if err != nil {
		return 0, fmt.Errorf("解析失败: %w", err)
	}

	// 收集统计信息
	st := collectStats(document)
	if *verbose {
		printDetail("发现: %d 个 struct, %d 个 enum, %d 个 const, %d 个 interface",
			st.structs, st.enums, st.consts, st.interfaces)
	}

	baseName := strings.TrimSuffix(filepath.Base(inputFile), filepath.Ext(inputFile))

	// 根据 langOutputs 生成代码
	generatedCount := 0
	for lang, outDir := range langOutputs {
		count, err := generateForLang(lang, outDir, document, baseName, st)
		if err != nil {
			return generatedCount, err
		}
		generatedCount += count

		displayName := langDisplayNames[lang]
		if displayName == "" {
			displayName = lang
		}

		absDir, _ := filepath.Abs(outDir)
		printGenerated("  → %s: %s (%d 个文件)", displayName, absDir, count)
	}

	return generatedCount, nil
}

// collectStats 收集 IDL 文件统计信息
func collectStats(document interface{}) stats {
	doc := document.(*ast.Document)
	st := stats{}

	for _, ns := range doc.Namespaces {
		st.structs += len(ns.Structs)
		st.enums += len(ns.Enums)
		st.consts += len(ns.Consts)
		st.interfaces += len(ns.Interfaces)
	}

	return st
}

// generateForLang 根据语言调用对应的代码生成器
func generateForLang(lang, outDir string, document interface{}, baseName string, st stats) (int, error) {
	doc := document.(*ast.Document)

	if *verbose {
		printDetail("生成 %s 代码...", langDisplayNames[lang])
	}

	// 创建输出目录
	if err := os.MkdirAll(outDir, 0755); err != nil {
		return 0, fmt.Errorf("创建输出目录失败: %w", err)
	}

	// 估算生成文件数
	estimatedFiles := 0

	switch lang {
	case "python":
		gen := python.NewGenerator(outDir)
		if err := gen.Generate(doc, baseName); err != nil {
			return 0, fmt.Errorf("生成 Python 代码失败: %w", err)
		}
		estimatedFiles = 1
	case "go":
		gen := golang.NewGenerator(outDir)
		if err := gen.Generate(doc, baseName); err != nil {
			return 0, fmt.Errorf("生成 Go 代码失败: %w", err)
		}
		estimatedFiles = len(doc.Namespaces)
	case "java":
		gen := java.NewGenerator(outDir)
		if err := gen.Generate(doc, baseName); err != nil {
			return 0, fmt.Errorf("生成 Java 代码失败: %w", err)
		}
		// Java 每个 struct/enum/interface 一个文件
		estimatedFiles = st.structs + st.enums + st.interfaces*3 // Prx, Servant, Callback
		if st.consts > 0 {
			estimatedFiles++
		}
	case "cpp":
		gen := cpp.NewGenerator(outDir)
		if err := gen.Generate(doc, baseName); err != nil {
			return 0, fmt.Errorf("生成 C++ 代码失败: %w", err)
		}
		estimatedFiles = 2 // .h and .cpp
	case "csharp":
		gen := csharp.NewGenerator(outDir)
		if err := gen.Generate(doc, baseName); err != nil {
			return 0, fmt.Errorf("生成 C# 代码失败: %w", err)
		}
		estimatedFiles = 1
	case "rust":
		gen := rust.NewGenerator(outDir)
		if err := gen.Generate(doc, baseName); err != nil {
			return 0, fmt.Errorf("生成 Rust 代码失败: %w", err)
		}
		estimatedFiles = 1
	case "php":
		gen := php.NewGenerator(outDir)
		if err := gen.Generate(doc, baseName); err != nil {
			return 0, fmt.Errorf("生成 PHP 代码失败: %w", err)
		}
		estimatedFiles = 1
	case "objc":
		gen := objc.NewGenerator(outDir)
		if err := gen.Generate(doc, baseName); err != nil {
			return 0, fmt.Errorf("生成 Objective-C 代码失败: %w", err)
		}
		estimatedFiles = 2 // .h and .m
	case "node":
		gen := node.NewGenerator(outDir)
		if err := gen.Generate(doc, baseName); err != nil {
			return 0, fmt.Errorf("生成 Node.js 代码失败: %w", err)
		}
		estimatedFiles = 1
	case "typescript":
		gen := typescript.NewGenerator(outDir)
		if err := gen.Generate(doc, baseName); err != nil {
			return 0, fmt.Errorf("生成 TypeScript 代码失败: %w", err)
		}
		estimatedFiles = 4 // types.ts, client.ts, server.ts, index.ts per namespace
	case "swift":
		gen := swift.NewGenerator(outDir)
		if err := gen.Generate(doc, baseName); err != nil {
			return 0, fmt.Errorf("生成 Swift 代码失败: %w", err)
		}
		// Swift: Types, Client (if has interfaces), Server (if has interfaces) per namespace
		estimatedFiles = len(doc.Namespaces)
		if st.interfaces > 0 {
			estimatedFiles *= 3
		}
	default:
		return 0, fmt.Errorf("未知语言: %s", lang)
	}

	return estimatedFiles, nil
}

// 输出辅助函数
func disableColors() {
	// 在 init 中设置全局变量可能更好，但这里简单处理
}

func printError(format string, args ...interface{}) {
	msg := fmt.Sprintf(format, args...)
	fmt.Fprintf(os.Stderr, "%s%s✗ 错误:%s %s\n", colorBold, colorRed, colorReset, msg)
}

func printWarning(format string, args ...interface{}) {
	msg := fmt.Sprintf(format, args...)
	fmt.Fprintf(os.Stderr, "%s⚠ 警告:%s %s\n", colorYellow, colorReset, msg)
}

func printSuccess(format string, args ...interface{}) {
	msg := fmt.Sprintf(format, args...)
	fmt.Printf("%s✓%s %s\n", colorGreen, colorReset, msg)
}

func printInfo(format string, args ...interface{}) {
	msg := fmt.Sprintf(format, args...)
	fmt.Printf("%sℹ%s %s\n", colorBlue, colorReset, msg)
}

func printStep(format string, args ...interface{}) {
	msg := fmt.Sprintf(format, args...)
	fmt.Printf("%s▶%s %s\n", colorCyan, colorReset, msg)
}

func printDetail(format string, args ...interface{}) {
	msg := fmt.Sprintf(format, args...)
	fmt.Printf("  %s\n", msg)
}

func printGenerated(format string, args ...interface{}) {
	msg := fmt.Sprintf(format, args...)
	fmt.Printf("%s%s\n", colorGreen, colorReset+msg)
}
