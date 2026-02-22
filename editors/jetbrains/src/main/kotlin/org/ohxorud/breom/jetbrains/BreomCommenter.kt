package org.ohxorud.breom.jetbrains

import com.intellij.lang.Commenter

class BreomCommenter : Commenter {
    override fun getLineCommentPrefix(): String = "// "
    override fun getBlockCommentPrefix(): String = "/*"
    override fun getBlockCommentSuffix(): String = "*/"
    override fun getCommentedBlockCommentPrefix(): String? = null
    override fun getCommentedBlockCommentSuffix(): String? = null
}
