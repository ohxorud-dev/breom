package org.ohxorud.breom.jetbrains.index

import com.intellij.openapi.fileTypes.FileTypeRegistry
import com.intellij.util.indexing.DataIndexer
import com.intellij.util.indexing.DefaultFileTypeSpecificInputFilter
import com.intellij.util.indexing.FileBasedIndex
import com.intellij.util.indexing.FileContent
import com.intellij.util.indexing.ID
import com.intellij.util.indexing.ScalarIndexExtension
import com.intellij.util.io.EnumeratorStringDescriptor
import org.ohxorud.breom.jetbrains.BreomFileType

class BreomModulePathIndex : ScalarIndexExtension<String>() {
    override fun getName(): ID<String, Void> = NAME

    override fun getIndexer(): DataIndexer<String, Void, FileContent> = DataIndexer { input ->
        if (isMetaFile(input.fileName)) {
            return@DataIndexer emptyMap()
        }
        val pkg = packageRegex.find(input.contentAsText)?.groupValues?.get(1)
        if (pkg.isNullOrBlank()) {
            emptyMap()
        } else {
            mapOf(pkg to null)
        }
    }

    override fun getKeyDescriptor() = EnumeratorStringDescriptor.INSTANCE

    override fun getInputFilter(): FileBasedIndex.InputFilter =
        DefaultFileTypeSpecificInputFilter(BreomFileType)

    override fun dependsOnFileContent(): Boolean = true

    override fun getVersion(): Int = 1

    private fun isMetaFile(fileName: String): Boolean {
        return fileName.equals("project.breom", ignoreCase = true)
            || fileName.equals("lock.breom", ignoreCase = true)
    }

    companion object {
        val NAME: ID<String, Void> = ID.create("breom.module.path.index")
        private val packageRegex = Regex("(?m)^\\s*package\\s+([A-Za-z_][A-Za-z0-9_.]*)\\s*$")
    }
}
