package org.ohxorud.breom.jetbrains.meta

enum class MetaSeverity {
    ERROR,
    WARNING,
}

data class MetaDiagnostic(
    val line: Int,
    val message: String,
    val severity: MetaSeverity = MetaSeverity.ERROR,
)

object BreomMetaValidators {
    private val semverRegex = Regex("^[0-9]+\\.[0-9]+\\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\\+[0-9A-Za-z.-]+)?$")
    private val packageRegex = Regex("^[A-Za-z_][A-Za-z0-9_]*(?:\\.[A-Za-z_][A-Za-z0-9_]*)*$")
    private val depRegex = Regex("^dep\\s+\"([^\"]+)\"\\s+\"([^\"]+)\"$")
    private val entrypointRegex = Regex("^[A-Za-z0-9_./\\-]+\\.brm$")

    fun validateProjectManifest(content: String, projectRootPath: String? = null): List<MetaDiagnostic> {
        val diagnostics = mutableListOf<MetaDiagnostic>()
        var hasBreom = false
        var hasPackage = false
        val deps = LinkedHashSet<Pair<String, String>>()

        content.lines().forEachIndexed { idx, lineRaw ->
            val line = lineRaw.trim()
            if (line.isBlank() || line.startsWith("//")) {
                return@forEachIndexed
            }

            when {
                line.startsWith("breom ") -> {
                    val version = line.removePrefix("breom ").trim()
                    if (!semverRegex.matches(version)) {
                        diagnostics += MetaDiagnostic(
                            idx,
                            "Invalid Breom version. Use semantic versioning, e.g. `breom 0.1.0`.",
                        )
                    }
                    hasBreom = true
                }

                line.startsWith("package ") -> {
                    val pkg = line.removePrefix("package ").trim()
                    if (!packageRegex.matches(pkg)) {
                        diagnostics += MetaDiagnostic(
                            idx,
                            "Invalid package name. Use dot-separated identifiers (example: `package app.core`).",
                        )
                    }
                    hasPackage = true
                }

                line.startsWith("entrypoint ") -> {
                    val entry = line.removePrefix("entrypoint ").trim().trim('"')
                    if (entry.isBlank() || !entrypointRegex.matches(entry)) {
                        diagnostics += MetaDiagnostic(
                            idx,
                            "Invalid entrypoint. It must point to a `.brm` file (example: `entrypoint main.brm`).",
                        )
                    } else if (entry.startsWith("/") || entry.startsWith("../") || entry.contains("..\\")) {
                        diagnostics += MetaDiagnostic(
                            idx,
                            "Invalid entrypoint path. Use a safe project-relative path without parent traversal.",
                        )
                    } else if (projectRootPath != null) {
                        val root = java.nio.file.Path.of(projectRootPath)
                        val candidate = root.resolve(entry).normalize()
                        if (!candidate.startsWith(root)) {
                            diagnostics += MetaDiagnostic(
                                idx,
                                "Entrypoint escapes the project directory. Keep it inside the current project.",
                            )
                        } else if (!java.nio.file.Files.exists(candidate)) {
                            diagnostics += MetaDiagnostic(
                                idx,
                                "Entrypoint file not found: `$entry`. Create the file or update the path.",
                            )
                        }
                    }
                }

                line.startsWith("dep ") -> {
                    val match = depRegex.matchEntire(line)
                    if (match == null) {
                        diagnostics += MetaDiagnostic(
                            idx,
                            "Invalid dependency declaration. Use `dep \"<repo>\" \"<tag>\"`.",
                        )
                    } else {
                        val repo = match.groupValues[1]
                        val tag = match.groupValues[2]
                        if (!repo.contains('/')) {
                            diagnostics += MetaDiagnostic(
                                idx,
                                "Dependency repository looks invalid. Expected a path-like repo (example: `owner/repo`).",
                            )
                        }
                        if (tag.isBlank()) {
                            diagnostics += MetaDiagnostic(idx, "Dependency tag cannot be empty.")
                        }
                        val depKey = repo to tag
                        if (!deps.add(depKey)) {
                            diagnostics += MetaDiagnostic(
                                idx,
                                "Duplicate dependency: `$repo` at tag `$tag` is declared more than once.",
                                MetaSeverity.WARNING,
                            )
                        }
                    }
                }

                line.startsWith("import ") -> {
                    diagnostics += MetaDiagnostic(
                        idx,
                        "`import` is not allowed in `project.breom` (line ${idx + 1}). Use `dep \"<repo>\" \"<tag>\"`.",
                    )
                }

                else -> diagnostics += MetaDiagnostic(
                    idx,
                    "Unknown declaration in `project.breom`. Allowed: `breom`, `package`, `entrypoint`, `dep`.",
                    MetaSeverity.WARNING,
                )
            }
        }

        if (!hasBreom) {
            diagnostics += MetaDiagnostic(0, "Missing required declaration: `breom <x.y.z>`.")
        }
        if (!hasPackage) {
            diagnostics += MetaDiagnostic(0, "Missing required declaration: `package <name>`.")
        }

        return diagnostics
    }

    fun validateLockFile(content: String): List<MetaDiagnostic> {
        val diagnostics = mutableListOf<MetaDiagnostic>()
        val trimmed = content.trim()
        if (trimmed.isEmpty()) {
            return diagnostics
        }
        if (!trimmed.startsWith("{") || !trimmed.endsWith("}")) {
            diagnostics += MetaDiagnostic(0, "`lock.breom` must be a JSON object.")
            return diagnostics
        }
        if (!content.contains("\"dependencies\"")) {
            diagnostics += MetaDiagnostic(0, "Missing required field: `\"dependencies\"` in `lock.breom`.")
        }
        if (content.contains("\"dependencies\"") && !Regex("\"dependencies\"\\s*:\\s*\\[").containsMatchIn(content)) {
            diagnostics += MetaDiagnostic(0, "Field `\"dependencies\"` must be an array.")
        }
        val dependencyObjects = Regex("\\{[^{}]*\\}").findAll(content).toList()
        for (obj in dependencyObjects) {
            val text = obj.value
            if (text.contains("\"repo\"") || text.contains("\"tag\"") || text.contains("\"commit\"")) {
                if (!text.contains("\"repo\"")) diagnostics += MetaDiagnostic(0, "Dependency entry is missing field `repo`.")
                if (!text.contains("\"tag\"")) diagnostics += MetaDiagnostic(0, "Dependency entry is missing field `tag`.")
                if (!text.contains("\"commit\"")) diagnostics += MetaDiagnostic(0, "Dependency entry is missing field `commit`.")
            }
        }
        return diagnostics
    }
}
