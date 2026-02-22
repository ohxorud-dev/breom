package org.ohxorud.breom.jetbrains

import com.intellij.ide.wizard.NewProjectWizardStep
import com.intellij.ide.wizard.language.LanguageGeneratorNewProjectWizard
import com.intellij.openapi.project.Project
import java.nio.file.Files
import java.nio.file.Path
import javax.swing.Icon

class BreomLanguageGeneratorNewProjectWizard : LanguageGeneratorNewProjectWizard {
    override val name: String = "Breom"

    override val icon: Icon = BreomIcons.FILE

    override fun createStep(parent: NewProjectWizardStep): NewProjectWizardStep {
        return object : NewProjectWizardStep by parent {
            override fun setupProject(project: Project) {
                createStarterFiles(project)
            }
        }
    }

    private fun createStarterFiles(project: Project) {
        val basePath = project.basePath ?: return
        val root = Path.of(basePath)

        val moduleName = root.fileName?.toString().orEmpty().ifBlank { "app" }
        val projectFile = root.resolve("project.breom")
        if (Files.notExists(projectFile)) {
            Files.writeString(projectFile, "breom 0.1.0\npackage $moduleName\n")
        }

        val mainFile = root.resolve("main.brm")
        if (Files.notExists(mainFile)) {
            Files.writeString(
                mainFile,
                "fn main() Int {\n    println(\"Hello, Breom!\")\n    return 0\n}\n",
            )
        }
    }
}
