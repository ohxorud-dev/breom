package org.ohxorud.breom.jetbrains.run

import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.execution.configurations.ConfigurationType
import com.intellij.execution.configurations.ConfigurationTypeUtil
import com.intellij.execution.configurations.RunConfiguration
import com.intellij.openapi.project.Project
import org.ohxorud.breom.jetbrains.BreomIcons
import javax.swing.Icon

class BreomRunConfigurationType : ConfigurationType {
    private val factory = BreomRunConfigurationFactory(this)

    override fun getDisplayName(): String = "Breom"

    override fun getConfigurationTypeDescription(): String = "Breom run configuration"

    override fun getIcon(): Icon = BreomIcons.FILE

    override fun getId(): String = "BreomRunConfiguration"

    override fun getConfigurationFactories(): Array<ConfigurationFactory> = arrayOf(factory)

    fun factory(): ConfigurationFactory = factory

    companion object {
        fun getInstance(): BreomRunConfigurationType {
            return ConfigurationTypeUtil.findConfigurationType(BreomRunConfigurationType::class.java)
        }
    }
}

private class BreomRunConfigurationFactory(type: ConfigurationType) : ConfigurationFactory(type) {
    override fun getId(): String = "BreomRunConfigurationFactory"

    override fun createTemplateConfiguration(project: Project): RunConfiguration {
        return BreomRunConfiguration(project, this, "Breom")
    }
}
