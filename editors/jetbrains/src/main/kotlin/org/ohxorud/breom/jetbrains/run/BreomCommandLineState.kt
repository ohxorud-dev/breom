package org.ohxorud.breom.jetbrains.run

import com.intellij.execution.ExecutionException
import com.intellij.execution.configurations.CommandLineState
import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.process.KillableProcessHandler
import com.intellij.execution.process.ProcessHandler
import com.intellij.execution.runners.ExecutionEnvironment
import java.nio.charset.StandardCharsets

class BreomCommandLineState(
    environment: ExecutionEnvironment,
    private val configuration: BreomRunConfiguration,
) : CommandLineState(environment) {
    @Throws(ExecutionException::class)
    override fun startProcess(): ProcessHandler {
        val executable = BreomRunSupport.resolveExecutable(environment.project, configuration.breomPath)
            ?: throw ExecutionException("BREOM_HOME is not set or BREOM_HOME/bin/breom was not found. Set Breom settings or BREOM_HOME.")
        val commandLine = GeneralCommandLine(executable)
            .withCharset(StandardCharsets.UTF_8)

        commandLine.addParameter("run")
        if (configuration.isFileTarget()) {
            commandLine.addParameter(configuration.filePath)
        }

        val cwd = configuration.resolvedWorkingDirectory()
        if (!cwd.isNullOrBlank()) {
            commandLine.withWorkDirectory(cwd)
        }

        return KillableProcessHandler(commandLine)
    }
}
