package org.ohxorud.breom.jetbrains.run

import com.intellij.execution.Executor
import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.execution.configurations.RunConfigurationBase
import com.intellij.execution.configurations.RunProfileState
import com.intellij.execution.configurations.RuntimeConfigurationError
import com.intellij.execution.configurations.RuntimeConfigurationException
import com.intellij.execution.configurations.RuntimeConfigurationWarning
import com.intellij.execution.runners.ExecutionEnvironment
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.JDOMExternalizerUtil
import org.jdom.Element
import java.io.File

class BreomRunConfiguration(
    project: Project,
    factory: ConfigurationFactory,
    name: String,
) : RunConfigurationBase<Any>(project, factory, name) {

    var runTarget: String = TARGET_PROJECT
    var filePath: String = ""
    var workingDirectory: String = ""
    var breomPath: String = ""

    override fun getConfigurationEditor() = BreomRunConfigurationEditor(project)

    override fun checkConfiguration() {
        if (runTarget == TARGET_FILE) {
            if (filePath.isBlank()) {
                throw RuntimeConfigurationError("Select a .brm file to run.")
            }
            if (!File(filePath).exists()) {
                throw RuntimeConfigurationError("File does not exist: $filePath")
            }
            if (!filePath.endsWith(".brm")) {
                throw RuntimeConfigurationWarning("Selected file is not a .brm file.")
            }
        }

        val cwd = resolvedWorkingDirectory()
        if (cwd.isNullOrBlank()) {
            throw RuntimeConfigurationError("Working directory is not set.")
        }
        if (!File(cwd).isDirectory) {
            throw RuntimeConfigurationError("Working directory does not exist: $cwd")
        }

        if (BreomRunSupport.resolveExecutable(project, breomPath).isNullOrBlank()) {
            throw RuntimeConfigurationError("Set BREOM_HOME and ensure BREOM_HOME/bin/breom exists, or set Breom executable path.")
        }
    }

    override fun getState(executor: Executor, environment: ExecutionEnvironment): RunProfileState {
        return BreomCommandLineState(environment, this)
    }

    fun isFileTarget(): Boolean = runTarget == TARGET_FILE

    fun resolvedWorkingDirectory(): String? {
        val explicit = workingDirectory.trim()
        if (explicit.isNotEmpty()) {
            return explicit
        }

        if (isFileTarget() && filePath.isNotBlank()) {
            return File(filePath).parent
        }

        return project.basePath
    }

    override fun readExternal(element: Element) {
        super.readExternal(element)
        runTarget = JDOMExternalizerUtil.readField(element, ATTR_TARGET) ?: TARGET_PROJECT
        filePath = JDOMExternalizerUtil.readField(element, ATTR_FILE_PATH) ?: ""
        workingDirectory = JDOMExternalizerUtil.readField(element, ATTR_WORK_DIR) ?: ""
        breomPath = JDOMExternalizerUtil.readField(element, ATTR_BREOM_PATH) ?: ""
    }

    override fun writeExternal(element: Element) {
        super.writeExternal(element)
        JDOMExternalizerUtil.writeField(element, ATTR_TARGET, runTarget)
        JDOMExternalizerUtil.writeField(element, ATTR_FILE_PATH, filePath)
        JDOMExternalizerUtil.writeField(element, ATTR_WORK_DIR, workingDirectory)
        JDOMExternalizerUtil.writeField(element, ATTR_BREOM_PATH, breomPath)
    }

    companion object {
        const val TARGET_PROJECT = "project"
        const val TARGET_FILE = "file"

        private const val ATTR_TARGET = "runTarget"
        private const val ATTR_FILE_PATH = "filePath"
        private const val ATTR_WORK_DIR = "workingDirectory"
        private const val ATTR_BREOM_PATH = "breomPath"
    }
}
