package org.ohxorud.breom.jetbrains.settings

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage

@State(name = "BreomSettings", storages = [Storage("breom.xml")])
class BreomSettingsService : PersistentStateComponent<BreomSettingsService.State> {
    data class State(
        var breomHomePath: String = "",
    )

    private var state = State()

    override fun getState(): State = state

    override fun loadState(state: State) {
        this.state = state
    }

    var breomHomePath: String
        get() = state.breomHomePath
        set(value) {
            state.breomHomePath = value.trim()
        }

    companion object {
        fun getInstanceOrNull(): BreomSettingsService? {
            return ApplicationManager.getApplication().getService(BreomSettingsService::class.java)
        }

        fun getInstance(): BreomSettingsService {
            return getInstanceOrNull() ?: error("BreomSettingsService is not available")
        }
    }
}
