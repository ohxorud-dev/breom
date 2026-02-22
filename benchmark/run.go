package main

import (
	"bufio"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"time"
)

var baseLangs = []string{"breom", "rust", "go", "python", "node", "cpp", "java"}

type langTarget struct {
	ID   string
	Base string
}

type row struct {
	Category  string
	Algorithm string
	Lang      string
	Mean      float64
	Median    float64
	Output    string
}

type benchCase struct {
	Category  string
	Algorithm string
	Lang      string
	Source    string
}

type runConfig struct {
	CategoryFilter string
	AlgoFilter     string
	LangFilter     string
	LangFilters    map[string]struct{}
	ExcludeLang    string
	ExcludeFilters map[string]struct{}
	MeasureMode    string
	Metric         string
	SplitProfiles  bool
	RustProfiles   string
	CppOptLevels   string
	PythonProfiles string
	Warmup         int
	Repeat         int
	BreomBin       string
	CPUCore        int
}

func main() {
	defer func() {
		r := recover()
		if r == nil {
			return
		}
		if err, ok := r.(error); ok {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		panic(r)
	}()

	cfg := parseFlags()

	benchDir, err := os.Getwd()
	if err != nil {
		fatal(err)
	}

	buildDir := filepath.Join(benchDir, ".build")
	tmpDir := filepath.Join(benchDir, ".tmp")

	if err := os.MkdirAll(buildDir, 0o755); err != nil {
		fatal(err)
	}
	if err := os.MkdirAll(tmpDir, 0o755); err != nil {
		fatal(err)
	}
	defer cleanupArtifacts(
		buildDir,
		tmpDir,
		filepath.Join(benchDir, "benchmark", ".build"),
		filepath.Join(benchDir, "benchmark", ".tmp"),
	)

	if cfg.BreomBin == "" {
		cfg.BreomBin = detectBreomBin(benchDir)
	}

	fixturePath := filepath.Join(tmpDir, "io_fixture.txt")
	if err := buildIOFixture(fixturePath); err != nil {
		fatal(err)
	}

	fmt.Printf("Running benchmarks (mode=%s metric=%s warmup=%d repeat=%d)\n", cfg.MeasureMode, cfg.Metric, cfg.Warmup, cfg.Repeat)
	if cfg.CPUCore >= 0 {
		fmt.Printf("CPU pinning requested: core=%d\n", cfg.CPUCore)
	}

	rows := make([]row, 0)
	cases := make([]benchCase, 0)
	langTargets, err := buildLangTargets(cfg)
	if err != nil {
		fatal(err)
	}

	categoryDirs, err := listSubdirs(benchDir)
	if err != nil {
		fatal(err)
	}

	for _, categoryDir := range categoryDirs {
		category := filepath.Base(categoryDir)
		if category == ".build" || category == ".tmp" || category == "benchmark" {
			continue
		}
		if cfg.CategoryFilter != "" && cfg.CategoryFilter != category {
			continue
		}

		algoDirs, err := listSubdirs(categoryDir)
		if err != nil {
			fatal(err)
		}

		for _, algoDir := range algoDirs {
			algorithm := filepath.Base(algoDir)
			if cfg.AlgoFilter != "" && cfg.AlgoFilter != algorithm {
				continue
			}

			for _, target := range langTargets {
				if !langAllowed(cfg.LangFilters, target) {
					continue
				}
				if langExcluded(cfg.ExcludeFilters, target) {
					continue
				}

				src := langSource(algoDir, target.Base)
				if src == "" {
					continue
				}
				if _, err := os.Stat(src); err != nil {
					if errors.Is(err, os.ErrNotExist) {
						continue
					}
					fatal(err)
				}

				if !langAvailable(target.ID, cfg.BreomBin) {
					fmt.Printf("skip: %-10s %-16s %-12s (runtime/tool missing)\n", category, algorithm, target.ID)
					continue
				}

				cases = append(cases, benchCase{
					Category:  category,
					Algorithm: algorithm,
					Lang:      target.ID,
					Source:    src,
				})
			}
		}
	}

	if len(cases) == 0 {
		fatal(fmt.Errorf("No benchmarks matched filters or available runtimes."))
	}

	if cfg.CPUCore >= 0 {
		if err := validateCPUPinningSupport(cfg.CPUCore); err != nil {
			fatal(err)
		}
	}

	preparedExecCmds := make(map[string][]string)
	if cfg.MeasureMode == "exec" {
		fmt.Println("Preparing build artifacts")
		for _, c := range cases {
			cmd, err := buildCommand(benchDir, buildDir, fixturePath, cfg.BreomBin, cfg.CPUCore, c.Category, c.Algorithm, c.Lang, c.Source)
			if err != nil {
				fatal(err)
			}
			preparedExecCmds[caseKey(c.Category, c.Algorithm, c.Lang)] = cmd
		}
		fmt.Println("Build preparation complete")
	}

	for _, c := range cases {
		result, unstable, err := runCase(
			benchDir,
			buildDir,
			fixturePath,
			cfg.BreomBin,
			cfg.MeasureMode,
			cfg.Warmup,
			cfg.Repeat,
			cfg.CPUCore,
			c.Category,
			c.Algorithm,
			c.Lang,
			c.Source,
			preparedExecCmds[caseKey(c.Category, c.Algorithm, c.Lang)],
		)
		if err != nil {
			fatal(err)
		}

		fmt.Printf("%-10s %-16s %-12s mean=%0.6fs median=%0.6fs out=%s\n",
			result.Category,
			result.Algorithm,
			result.Lang,
			result.Mean,
			result.Median,
			result.Output,
		)

		if unstable {
			fmt.Printf("WARNING: %-10s %-16s %-12s output changed across repeats\n", c.Category, c.Algorithm, c.Lang)
		}

		rows = append(rows, result)
	}

	fmt.Println()
	fmt.Println("Summary")
	printGroupedSummary(rows, cfg.Metric)
	printQuickView(rows, cfg.Metric)

	printCrossLanguageMismatchNotice(rows)
}

func parseFlags() runConfig {
	var cfg runConfig

	flag.StringVar(&cfg.CategoryFilter, "category", "", "run only one category")
	flag.StringVar(&cfg.AlgoFilter, "algorithm", "", "run only one algorithm")
	flag.StringVar(&cfg.LangFilter, "lang", "", "run one or more languages (comma-separated)")
	flag.StringVar(&cfg.ExcludeLang, "exclude-lang", "", "exclude one or more languages (comma-separated)")
	flag.StringVar(&cfg.MeasureMode, "mode", "exec", "measurement mode: exec (run-only after one build) or total (build+run)")
	flag.StringVar(&cfg.Metric, "metric", "median", "summary metric: median (default) or mean")
	flag.BoolVar(&cfg.SplitProfiles, "split-profiles", false, "expand rust/cpp/python into profile variants")
	flag.StringVar(&cfg.RustProfiles, "rust-profiles", "dev,release,release-native,release-lto,release-lto-native,size,size-z", "comma-separated rust profiles when --split-profiles is set")
	flag.StringVar(&cfg.CppOptLevels, "cpp-opt-levels", "O0,O1,O2,O3,Ofast,Os,Oz", "comma-separated cpp optimization profiles when --split-profiles is set")
	flag.StringVar(&cfg.PythonProfiles, "python-profiles", "cpython,cpython-O,cpython-OO,pypy,pypy-O,pypy-OO", "comma-separated python runtimes/profiles when --split-profiles is set")
	flag.IntVar(&cfg.Warmup, "warmup", 1, "warmup runs per case")
	flag.IntVar(&cfg.Repeat, "repeat", 3, "measured runs per case")
	flag.StringVar(&cfg.BreomBin, "breom-bin", "", "path to breom binary (optional)")
	flag.IntVar(&cfg.CPUCore, "cpu-core", -1, "pin benchmark subprocesses to one CPU core (linux only, requires taskset)")
	flag.Parse()

	if cfg.Warmup < 0 {
		fatal(fmt.Errorf("warmup must be >= 0"))
	}
	if cfg.Repeat <= 0 {
		fatal(fmt.Errorf("repeat must be > 0"))
	}
	if cfg.MeasureMode != "total" && cfg.MeasureMode != "exec" {
		fatal(fmt.Errorf("mode must be one of: total, exec"))
	}
	if cfg.Metric != "median" && cfg.Metric != "mean" {
		fatal(fmt.Errorf("metric must be one of: median, mean"))
	}
	cfg.LangFilters = parseLangFilters(cfg.LangFilter)
	cfg.ExcludeFilters = parseExcludeLangFilters(cfg.ExcludeLang)
	if _, err := buildLangTargets(cfg); err != nil {
		fatal(err)
	}
	if cfg.CPUCore < -1 {
		fatal(fmt.Errorf("cpu-core must be >= -1"))
	}

	return cfg
}

func validateCPUPinningSupport(cpuCore int) error {
	if cpuCore < 0 {
		return nil
	}
	if runtime.GOOS != "linux" {
		return fmt.Errorf("cpu pinning is currently supported only on linux (requested core=%d on %s)", cpuCore, runtime.GOOS)
	}
	if _, err := exec.LookPath("taskset"); err != nil {
		return fmt.Errorf("cpu pinning requested but taskset was not found in PATH")
	}
	return nil
}

func parseLangFilters(raw string) map[string]struct{} {
	filters := make(map[string]struct{})
	for _, token := range strings.Split(raw, ",") {
		name := strings.ToLower(strings.TrimSpace(token))
		if name == "" {
			continue
		}
		filters[name] = struct{}{}
	}
	return filters
}

func parseExcludeLangFilters(raw string) map[string]struct{} {
	filters := make(map[string]struct{})
	for _, token := range strings.Split(raw, ",") {
		name := strings.ToLower(strings.TrimSpace(token))
		if name == "" {
			continue
		}
		if strings.HasPrefix(name, "rust-") {
			name = "rust"
		} else if strings.HasPrefix(name, "cpp-") {
			name = "cpp"
		} else if strings.HasPrefix(name, "python-") {
			name = "python"
		} else if strings.HasPrefix(name, "pypy") {
			name = "python"
		}
		filters[name] = struct{}{}
	}
	return filters
}

func langAllowed(filters map[string]struct{}, target langTarget) bool {
	if len(filters) == 0 {
		return true
	}
	return langMatches(filters, target)
}

func langExcluded(filters map[string]struct{}, target langTarget) bool {
	if len(filters) == 0 {
		return false
	}
	_, ok := filters[strings.ToLower(target.Base)]
	return ok
}

func langMatches(filters map[string]struct{}, target langTarget) bool {
	_, okID := filters[strings.ToLower(target.ID)]
	if okID {
		return true
	}
	_, okBase := filters[strings.ToLower(target.Base)]
	return okBase
}

func buildLangTargets(cfg runConfig) ([]langTarget, error) {
	if !cfg.SplitProfiles {
		targets := make([]langTarget, 0, len(baseLangs))
		for _, lang := range baseLangs {
			targets = append(targets, langTarget{ID: lang, Base: lang})
		}
		return targets, nil
	}

	rustProfiles, err := parseRustProfiles(cfg.RustProfiles)
	if err != nil {
		return nil, err
	}
	cppLevels, err := parseCppOptLevels(cfg.CppOptLevels)
	if err != nil {
		return nil, err
	}
	pythonProfiles, err := parsePythonProfiles(cfg.PythonProfiles)
	if err != nil {
		return nil, err
	}

	targets := make([]langTarget, 0, len(baseLangs)+len(rustProfiles)+len(cppLevels)+len(pythonProfiles))
	for _, lang := range baseLangs {
		switch lang {
		case "rust":
			for _, profile := range rustProfiles {
				targets = append(targets, langTarget{ID: "rust-" + profile, Base: "rust"})
			}
		case "cpp":
			for _, level := range cppLevels {
				targets = append(targets, langTarget{ID: "cpp-" + level, Base: "cpp"})
			}
		case "python":
			for _, profile := range pythonProfiles {
				targets = append(targets, langTarget{ID: "python-" + profile, Base: "python"})
			}
		default:
			targets = append(targets, langTarget{ID: lang, Base: lang})
		}
	}

	return targets, nil
}

func parseRustProfiles(raw string) ([]string, error) {
	parts := strings.Split(raw, ",")
	seen := make(map[string]struct{})
	out := make([]string, 0, len(parts))
	for _, p := range parts {
		profile := strings.TrimSpace(strings.ToLower(p))
		if profile == "" {
			continue
		}
		if profile != "dev" && profile != "release" && profile != "release-lto" && profile != "release-native" && profile != "release-lto-native" && profile != "size" && profile != "size-z" {
			return nil, fmt.Errorf("invalid rust profile: %q (allowed: dev, release, release-lto, release-native, release-lto-native, size, size-z)", profile)
		}
		if _, ok := seen[profile]; ok {
			continue
		}
		seen[profile] = struct{}{}
		out = append(out, profile)
	}
	if len(out) == 0 {
		return nil, fmt.Errorf("rust-profiles must include at least one profile")
	}
	return out, nil
}

func parseCppOptLevels(raw string) ([]string, error) {
	parts := strings.Split(raw, ",")
	seen := make(map[string]struct{})
	out := make([]string, 0, len(parts))
	for _, p := range parts {
		level := strings.TrimSpace(strings.ToUpper(p))
		if level == "" {
			continue
		}
		if level != "O0" && level != "O1" && level != "O2" && level != "O3" && level != "OFAST" && level != "OS" && level != "OZ" {
			return nil, fmt.Errorf("invalid cpp opt level: %q (allowed: O0, O1, O2, O3, Ofast, Os, Oz)", level)
		}
		if _, ok := seen[level]; ok {
			continue
		}
		seen[level] = struct{}{}
		switch level {
		case "OFAST":
			out = append(out, "Ofast")
		case "OS":
			out = append(out, "Os")
		case "OZ":
			out = append(out, "Oz")
		default:
			out = append(out, level)
		}
	}
	if len(out) == 0 {
		return nil, fmt.Errorf("cpp-opt-levels must include at least one level")
	}
	return out, nil
}

func parsePythonProfiles(raw string) ([]string, error) {
	parts := strings.Split(raw, ",")
	seen := make(map[string]struct{})
	out := make([]string, 0, len(parts))
	for _, p := range parts {
		profile := strings.TrimSpace(strings.ToLower(p))
		if profile == "" {
			continue
		}
		if profile != "cpython" && profile != "cpython-o" && profile != "cpython-oo" && profile != "pypy" && profile != "pypy-o" && profile != "pypy-oo" {
			return nil, fmt.Errorf("invalid python profile: %q (allowed: cpython, cpython-O, cpython-OO, pypy, pypy-O, pypy-OO)", profile)
		}
		if _, ok := seen[profile]; ok {
			continue
		}
		seen[profile] = struct{}{}
		switch profile {
		case "cpython-o":
			out = append(out, "cpython-O")
		case "cpython-oo":
			out = append(out, "cpython-OO")
		case "pypy-o":
			out = append(out, "pypy-O")
		case "pypy-oo":
			out = append(out, "pypy-OO")
		default:
			out = append(out, profile)
		}
	}
	if len(out) == 0 {
		return nil, fmt.Errorf("python-profiles must include at least one profile")
	}
	return out, nil
}

func rustProfileForLang(lang string) string {
	if lang == "rust-dev" {
		return "dev"
	}
	if strings.HasPrefix(lang, "rust-") {
		return strings.TrimPrefix(lang, "rust-")
	}
	if lang == "rust" {
		return "release"
	}
	return ""
}

func cppOptLevelForLang(lang string) string {
	if strings.HasPrefix(lang, "cpp-") {
		level := strings.TrimPrefix(lang, "cpp-")
		level = strings.ToUpper(level)
		if level == "O0" || level == "O1" || level == "O2" || level == "O3" || level == "OFAST" || level == "OS" || level == "OZ" {
			if level == "OFAST" {
				return "Ofast"
			}
			if level == "OS" {
				return "Os"
			}
			if level == "OZ" {
				return "Oz"
			}
			return level
		}
	}
	if lang == "cpp" {
		return "O2"
	}
	return ""
}

func pythonProfileForLang(lang string) string {
	if strings.HasPrefix(lang, "python-") {
		return strings.TrimPrefix(lang, "python-")
	}
	if lang == "python" {
		return "cpython"
	}
	return ""
}

func detectBreomBin(benchDir string) string {
	candidates := []string{
		filepath.Clean(filepath.Join(benchDir, "..", "target", "release", "breom")),
		filepath.Join(benchDir, "target", "release", "breom"),
		filepath.Clean(filepath.Join(benchDir, "..", "target", "debug", "breom")),
		filepath.Join(benchDir, "target", "debug", "breom"),
	}

	for _, p := range candidates {
		st, err := os.Stat(p)
		if err != nil {
			continue
		}
		if st.Mode()&0o111 != 0 {
			return p
		}
	}

	return candidates[0]
}

func fatal(err error) {
	panic(err)
}

func cleanupArtifacts(paths ...string) {
	for _, p := range paths {
		if err := os.RemoveAll(p); err != nil {
			fmt.Fprintf(os.Stderr, "warning: failed to clean %s: %v\n", p, err)
		}
	}
}

func listSubdirs(path string) ([]string, error) {
	entries, err := os.ReadDir(path)
	if err != nil {
		return nil, err
	}
	dirs := make([]string, 0)
	for _, e := range entries {
		if e.IsDir() {
			dirs = append(dirs, filepath.Join(path, e.Name()))
		}
	}
	sort.Strings(dirs)
	return dirs, nil
}

func langSource(algoDir, lang string) string {
	switch lang {
	case "breom":
		return filepath.Join(algoDir, "main.brm")
	case "rust":
		return filepath.Join(algoDir, "main.rs")
	case "go":
		return filepath.Join(algoDir, "main.go")
	case "python":
		return filepath.Join(algoDir, "main.py")
	case "node":
		return filepath.Join(algoDir, "main.js")
	case "cpp":
		return filepath.Join(algoDir, "main.cpp")
	case "java":
		return filepath.Join(algoDir, "Main.java")
	default:
		return ""
	}
}

func langAvailable(lang, breomBin string) bool {
	if strings.HasPrefix(lang, "rust-") {
		lang = "rust"
	}
	if strings.HasPrefix(lang, "cpp-") {
		lang = "cpp"
	}
	if strings.HasPrefix(lang, "python-") {
		profile := pythonProfileForLang(lang)
		if strings.HasPrefix(profile, "pypy") {
			_, err := exec.LookPath("pypy3")
			return err == nil
		}
		lang = "python"
	}

	switch lang {
	case "breom":
		st, err := os.Stat(breomBin)
		if err != nil {
			return false
		}
		return st.Mode()&0o111 != 0
	case "rust":
		_, err := exec.LookPath("rustc")
		return err == nil
	case "go":
		_, err := exec.LookPath("go")
		return err == nil
	case "python":
		_, err := exec.LookPath("python3")
		return err == nil
	case "node":
		_, err := exec.LookPath("node")
		return err == nil
	case "cpp":
		_, err := exec.LookPath("c++")
		return err == nil
	case "java":
		_, javacErr := exec.LookPath("javac")
		if javacErr != nil {
			return false
		}
		_, javaErr := exec.LookPath("java")
		return javaErr == nil
	default:
		return false
	}
}

func caseKey(category, algorithm, lang string) string {
	return category + "\x00" + algorithm + "\x00" + lang
}

func runCase(benchDir, buildDir, fixturePath, breomBin, measureMode string, warmupRuns, measuredRuns, cpuCore int, category, algorithm, lang, src string, preparedExecCmd []string) (row, bool, error) {
	var cmd []string
	var err error

	if measureMode == "exec" {
		if len(preparedExecCmd) > 0 {
			cmd = preparedExecCmd
		} else {
			cmd, err = buildCommand(benchDir, buildDir, fixturePath, breomBin, cpuCore, category, algorithm, lang, src)
			if err != nil {
				return row{}, false, err
			}
		}
	}

	runOnce := func() (float64, string, error) {
		if measureMode == "exec" {
			return measureCommand(cmd, cpuCore)
		}

		start := time.Now()
		caseCmd, err := buildCommand(benchDir, buildDir, fixturePath, breomBin, cpuCore, category, algorithm, lang, src)
		if err != nil {
			return 0, "", err
		}
		out, err := executeCommand(caseCmd, cpuCore)
		if err != nil {
			return 0, "", err
		}
		return time.Since(start).Seconds(), out, nil
	}

	for i := 0; i < warmupRuns; i++ {
		if _, _, err := runOnce(); err != nil {
			return row{}, false, err
		}
	}

	times := make([]float64, 0, measuredRuns)
	firstOut := ""
	unstable := false

	for i := 0; i < measuredRuns; i++ {
		elapsed, out, err := runOnce()
		if err != nil {
			return row{}, false, err
		}
		times = append(times, elapsed)
		if i == 0 {
			firstOut = out
		} else if out != firstOut {
			unstable = true
		}
	}

	mean, median := stats(times)
	return row{
		Category:  category,
		Algorithm: algorithm,
		Lang:      lang,
		Mean:      mean,
		Median:    median,
		Output:    firstOut,
	}, unstable, nil
}

func buildCommand(benchDir, buildDir, fixturePath, breomBin string, cpuCore int, category, algorithm, lang, src string) ([]string, error) {
	rustProfile := rustProfileForLang(lang)
	cppOptLevel := cppOptLevelForLang(lang)
	pythonProfile := pythonProfileForLang(lang)
	baseLang := lang
	if rustProfile != "" {
		baseLang = "rust"
	}
	if cppOptLevel != "" {
		baseLang = "cpp"
	}
	if pythonProfile != "" {
		baseLang = "python"
	}

	switch baseLang {
	case "breom":
		cmd := []string{breomBin, "run", src}
		if category == "file_io" {
			cmd = append(cmd, fixturePath)
		}
		return cmd, nil
	case "rust":
		if rustProfile == "" {
			rustProfile = "release"
		}
		bin := filepath.Join(buildDir, fmt.Sprintf("%s_%s_%s", category, algorithm, strings.ReplaceAll(lang, "-", "_")))
		rustcArgs := []string{}
		switch rustProfile {
		case "dev":
			rustcArgs = append(rustcArgs, "-C", "opt-level=0")
		case "release":
			rustcArgs = append(rustcArgs, "-O")
		case "release-lto":
			rustcArgs = append(rustcArgs, "-O", "-C", "lto=fat")
		case "release-native":
			rustcArgs = append(rustcArgs, "-O", "-C", "target-cpu=native")
		case "release-lto-native":
			rustcArgs = append(rustcArgs, "-O", "-C", "lto=fat", "-C", "target-cpu=native")
		case "size":
			rustcArgs = append(rustcArgs, "-C", "opt-level=s")
		case "size-z":
			rustcArgs = append(rustcArgs, "-C", "opt-level=z")
		default:
			return nil, fmt.Errorf("unsupported rust profile: %s", rustProfile)
		}
		rustcArgs = append(rustcArgs, src, "-o", bin)
		if err := runSimpleCommand(benchDir, cpuCore, "rustc", rustcArgs...); err != nil {
			return nil, err
		}
		cmd := []string{bin}
		if category == "file_io" {
			cmd = append(cmd, fixturePath)
		}
		return cmd, nil
	case "go":
		bin := filepath.Join(buildDir, fmt.Sprintf("%s_%s_go", category, algorithm))
		if err := runSimpleCommand(benchDir, cpuCore, "go", "build", "-o", bin, src); err != nil {
			return nil, err
		}
		cmd := []string{bin}
		if category == "file_io" {
			cmd = append(cmd, fixturePath)
		}
		return cmd, nil
	case "python":
		if pythonProfile == "" {
			pythonProfile = "cpython"
		}
		execName := "python3"
		runtimeArgs := []string{}
		switch pythonProfile {
		case "cpython":
		case "cpython-O":
			runtimeArgs = append(runtimeArgs, "-O")
		case "cpython-OO":
			runtimeArgs = append(runtimeArgs, "-OO")
		case "pypy":
			execName = "pypy3"
		case "pypy-O":
			execName = "pypy3"
			runtimeArgs = append(runtimeArgs, "-O")
		case "pypy-OO":
			execName = "pypy3"
			runtimeArgs = append(runtimeArgs, "-OO")
		default:
			return nil, fmt.Errorf("unsupported python profile: %s", pythonProfile)
		}
		cmd := []string{execName}
		cmd = append(cmd, runtimeArgs...)
		cmd = append(cmd, src)
		if category == "file_io" {
			cmd = append(cmd, fixturePath)
		}
		return cmd, nil
	case "node":
		cmd := []string{"node", src}
		if category == "file_io" {
			cmd = append(cmd, fixturePath)
		}
		return cmd, nil
	case "cpp":
		if cppOptLevel == "" {
			cppOptLevel = "O2"
		}
		bin := filepath.Join(buildDir, fmt.Sprintf("%s_%s_%s", category, algorithm, strings.ReplaceAll(lang, "-", "_")))
		if err := runSimpleCommand(benchDir, cpuCore, "c++", "-"+cppOptLevel, "-std=c++17", src, "-o", bin); err != nil {
			return nil, err
		}
		cmd := []string{bin}
		if category == "file_io" {
			cmd = append(cmd, fixturePath)
		}
		return cmd, nil
	case "java":
		classDir := filepath.Join(buildDir, fmt.Sprintf("%s_%s_java", category, algorithm))
		if err := os.MkdirAll(classDir, 0o755); err != nil {
			return nil, err
		}
		if err := runSimpleCommand(benchDir, cpuCore, "javac", "-d", classDir, src); err != nil {
			return nil, err
		}
		cmd := []string{"java", "-cp", classDir, "Main"}
		if category == "file_io" {
			cmd = append(cmd, fixturePath)
		}
		return cmd, nil
	default:
		return nil, fmt.Errorf("unsupported language: %s", lang)
	}
}

func runSimpleCommand(workdir string, cpuCore int, name string, args ...string) error {
	command := append([]string{name}, args...)
	command, err := maybePinCommand(command, cpuCore)
	if err != nil {
		return err
	}
	cmd := exec.Command(command[0], command[1:]...)
	cmd.Dir = workdir
	out, err := cmd.CombinedOutput()
	if err != nil {
		output := strings.TrimSpace(string(out))
		if output != "" {
			return fmt.Errorf("%s failed: %w\n%s", strings.Join(command, " "), err, output)
		}
		return fmt.Errorf("%s failed: %w", strings.Join(command, " "), err)
	}
	return nil
}

func measureCommand(cmdArgs []string, cpuCore int) (float64, string, error) {
	start := time.Now()
	outText, err := executeCommand(cmdArgs, cpuCore)
	elapsed := time.Since(start).Seconds()
	if err != nil {
		return 0, "", err
	}

	return elapsed, outText, nil
}

func executeCommand(cmdArgs []string, cpuCore int) (string, error) {
	if len(cmdArgs) == 0 {
		return "", fmt.Errorf("empty command")
	}
	pinnedCmdArgs, err := maybePinCommand(cmdArgs, cpuCore)
	if err != nil {
		return "", err
	}
	cmd := exec.Command(pinnedCmdArgs[0], pinnedCmdArgs[1:]...)

	var stdoutBuf strings.Builder
	var stderrBuf strings.Builder
	cmd.Stdout = &stdoutBuf
	cmd.Stderr = &stderrBuf
	err = cmd.Run()

	outText := strings.TrimSpace(stdoutBuf.String())
	outText = strings.ReplaceAll(outText, "\n", " ")

	if err != nil {
		errText := strings.TrimSpace(stderrBuf.String())
		if errText != "" {
			return "", fmt.Errorf("command failed: %s\n%s", strings.Join(pinnedCmdArgs, " "), errText)
		}
		return "", fmt.Errorf("command failed: %s (%w)", strings.Join(pinnedCmdArgs, " "), err)
	}

	return outText, nil
}

func maybePinCommand(cmdArgs []string, cpuCore int) ([]string, error) {
	if cpuCore < 0 {
		return cmdArgs, nil
	}
	if runtime.GOOS != "linux" {
		return nil, fmt.Errorf("cpu pinning is currently supported only on linux (requested core=%d on %s)", cpuCore, runtime.GOOS)
	}
	if _, err := exec.LookPath("taskset"); err != nil {
		return nil, fmt.Errorf("cpu pinning requested but taskset was not found in PATH")
	}
	pinned := make([]string, 0, len(cmdArgs)+3)
	pinned = append(pinned, "taskset", "-c", fmt.Sprintf("%d", cpuCore))
	pinned = append(pinned, cmdArgs...)
	return pinned, nil
}

func stats(values []float64) (float64, float64) {
	sorted := make([]float64, len(values))
	copy(sorted, values)
	sort.Float64s(sorted)

	mean := 0.0
	for _, v := range values {
		mean += v
	}
	mean /= float64(len(values))

	median := 0.0
	n := len(sorted)
	if n%2 == 1 {
		median = sorted[n/2]
	} else {
		median = (sorted[n/2-1] + sorted[n/2]) / 2
	}

	return mean, median
}

func buildIOFixture(path string) error {
	if _, err := os.Stat(path); err == nil {
		return nil
	}

	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()

	w := bufio.NewWriterSize(f, 1<<20)
	for i := 0; i < 150000; i++ {
		if _, err := io.WriteString(w, fmt.Sprintf("line-%06d value=%d\n", i, (i*17+11)%100000)); err != nil {
			return err
		}
	}

	if err := w.Flush(); err != nil {
		return err
	}

	return nil
}

func metricValue(r row, metric string) float64 {
	if metric == "mean" {
		return r.Mean
	}
	return r.Median
}

func printGroupedSummary(rows []row, metric string) {
	type key struct {
		Category  string
		Algorithm string
	}
	type langMean struct {
		Lang string
		Time float64
	}

	grouped := make(map[key]map[string]float64)
	for _, r := range rows {
		k := key{Category: r.Category, Algorithm: r.Algorithm}
		if _, ok := grouped[k]; !ok {
			grouped[k] = make(map[string]float64)
		}
		grouped[k][r.Lang] = metricValue(r, metric)
	}

	keys := make([]key, 0, len(grouped))
	for k := range grouped {
		keys = append(keys, k)
	}
	sort.Slice(keys, func(i, j int) bool {
		if keys[i].Category == keys[j].Category {
			return keys[i].Algorithm < keys[j].Algorithm
		}
		return keys[i].Category < keys[j].Category
	})

	fmt.Printf("metric=%s\n", metric)

	for _, k := range keys {
		timesByLang := grouped[k]
		entries := make([]langMean, 0, len(timesByLang))
		for lang, elapsed := range timesByLang {
			entries = append(entries, langMean{Lang: lang, Time: elapsed})
		}
		sort.Slice(entries, func(i, j int) bool {
			if entries[i].Time == entries[j].Time {
				return entries[i].Lang < entries[j].Lang
			}
			return entries[i].Time < entries[j].Time
		})

		parts := make([]string, 0, len(entries))
		fastest := entries[0].Time
		for i, entry := range entries {
			ratio := 1.0
			if fastest > 0 {
				ratio = entry.Time / fastest
			}
			parts = append(parts, fmt.Sprintf("%d) %s %.6fs (%.2fx)", i+1, entry.Lang, entry.Time, ratio))
		}

		fmt.Printf("%s/%s -> %s\n", k.Category, k.Algorithm, strings.Join(parts, ", "))
	}
}

func printQuickView(rows []row, metric string) {
	type key struct {
		Category  string
		Algorithm string
	}
	type langStats struct {
		Wins       int
		RatioSum   float64
		BenchSeen  int
		BestRatio  float64
		WorstRatio float64
		BestCase   string
		WorstCase  string
	}
	type langRank struct {
		Lang       string
		Wins       int
		AvgRatio   float64
		BestRatio  float64
		WorstRatio float64
		BestCase   string
		WorstCase  string
	}

	grouped := make(map[key][]row)
	for _, r := range rows {
		k := key{Category: r.Category, Algorithm: r.Algorithm}
		grouped[k] = append(grouped[k], r)
	}

	if len(grouped) == 0 {
		return
	}

	stats := make(map[string]*langStats)

	for k, benchRows := range grouped {
		if len(benchRows) == 0 {
			continue
		}

		fastest := metricValue(benchRows[0], metric)
		winner := benchRows[0].Lang
		for _, r := range benchRows[1:] {
			elapsed := metricValue(r, metric)
			if elapsed < fastest || (elapsed == fastest && r.Lang < winner) {
				fastest = elapsed
				winner = r.Lang
			}
		}

		for _, r := range benchRows {
			elapsed := metricValue(r, metric)
			ratio := 1.0
			if fastest > 0 {
				ratio = elapsed / fastest
			}
			caseName := fmt.Sprintf("%s/%s", k.Category, k.Algorithm)

			st, ok := stats[r.Lang]
			if !ok {
				st = &langStats{
					BestRatio:  ratio,
					WorstRatio: ratio,
					BestCase:   caseName,
					WorstCase:  caseName,
				}
				stats[r.Lang] = st
			} else {
				if ratio < st.BestRatio {
					st.BestRatio = ratio
					st.BestCase = caseName
				}
				if ratio > st.WorstRatio {
					st.WorstRatio = ratio
					st.WorstCase = caseName
				}
			}
			if r.Lang == winner {
				st.Wins++
			}
			st.RatioSum += ratio
			st.BenchSeen++
		}
	}

	ranking := make([]langRank, 0, len(stats))
	for lang, st := range stats {
		avgRatio := 1.0
		if st.BenchSeen > 0 {
			avgRatio = st.RatioSum / float64(st.BenchSeen)
		}
		ranking = append(ranking, langRank{
			Lang:       lang,
			Wins:       st.Wins,
			AvgRatio:   avgRatio,
			BestRatio:  st.BestRatio,
			WorstRatio: st.WorstRatio,
			BestCase:   st.BestCase,
			WorstCase:  st.WorstCase,
		})
	}

	sort.Slice(ranking, func(i, j int) bool {
		if ranking[i].Wins == ranking[j].Wins {
			if ranking[i].AvgRatio == ranking[j].AvgRatio {
				return ranking[i].Lang < ranking[j].Lang
			}
			return ranking[i].AvgRatio < ranking[j].AvgRatio
		}
		return ranking[i].Wins > ranking[j].Wins
	})

	fmt.Println()
	fmt.Println("Quick View")
	fmt.Printf("benchmarks=%d languages=%d metric=%s\n", len(grouped), len(ranking), metric)

	for _, r := range ranking {
		fmt.Printf("%s: wins=%d avg=%.2fx best=%s (%.2fx) worst=%s (%.2fx)\n",
			r.Lang,
			r.Wins,
			r.AvgRatio,
			r.BestCase,
			r.BestRatio,
			r.WorstCase,
			r.WorstRatio,
		)
	}
}

func printCrossLanguageMismatchNotice(rows []row) {
	type key struct {
		Category  string
		Algorithm string
	}

	grouped := make(map[key]map[string]string)
	for _, r := range rows {
		k := key{Category: r.Category, Algorithm: r.Algorithm}
		if _, ok := grouped[k]; !ok {
			grouped[k] = make(map[string]string)
		}
		grouped[k][r.Lang] = r.Output
	}

	keys := make([]key, 0, len(grouped))
	for k := range grouped {
		keys = append(keys, k)
	}
	sort.Slice(keys, func(i, j int) bool {
		if keys[i].Category == keys[j].Category {
			return keys[i].Algorithm < keys[j].Algorithm
		}
		return keys[i].Category < keys[j].Category
	})

	hasMismatch := false
	lines := make([]string, 0)

	for _, k := range keys {
		outByLang := grouped[k]
		unique := make(map[string]struct{})
		for _, out := range outByLang {
			unique[out] = struct{}{}
		}
		if len(unique) <= 1 {
			continue
		}
		hasMismatch = true

		parts := make([]string, 0, len(outByLang))
		langKeys := make([]string, 0, len(outByLang))
		for lang := range outByLang {
			langKeys = append(langKeys, lang)
		}
		sort.Strings(langKeys)
		for _, lang := range langKeys {
			parts = append(parts, fmt.Sprintf("%s=%s", lang, outByLang[lang]))
		}

		lines = append(lines, fmt.Sprintf("- %s/%s: %s", k.Category, k.Algorithm, strings.Join(parts, ", ")))
	}

	if hasMismatch {
		fmt.Println()
		fmt.Println("WARNING: output mismatch across languages")
		for _, line := range lines {
			fmt.Println(line)
		}
	}
}
