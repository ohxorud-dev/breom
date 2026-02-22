package org.ohxorud.breom.jetbrains

import com.intellij.ide.util.projectWizard.WizardContext
import com.intellij.ide.wizard.GeneratorNewProjectWizard
import com.intellij.ide.wizard.NewProjectWizardStep
import com.intellij.ide.wizard.language.EmptyProjectGeneratorNewProjectWizard
import com.intellij.openapi.project.Project
import java.io.IOException
import java.nio.file.Files
import java.nio.file.Path
import javax.swing.Icon

class BreomNewProjectWizardGenerator : GeneratorNewProjectWizard {
    override val id: String = "breom"

    override val name: String = "Breom"

    override val icon: Icon = BreomIcons.FILE

    override val description: String = "Starter Breom project with project.breom and main.brm"

    override fun createStep(context: WizardContext): NewProjectWizardStep {
        val base = EmptyProjectGeneratorNewProjectWizard().createStep(context)
        return object : NewProjectWizardStep by base {
            override fun setupProject(project: Project) {
                base.setupProject(project)
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

        removeImlFiles(root)
    }

    private fun removeImlFiles(root: Path) {
        try {
            Files.newDirectoryStream(root, "*.iml").use { stream ->
                for (file in stream) {
                    Files.deleteIfExists(file)
                }
            }
        } catch (_: IOException) {
        }
    }
}
