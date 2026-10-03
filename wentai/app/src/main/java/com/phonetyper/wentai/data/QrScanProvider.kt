package com.phonetyper.wentai.data

import android.content.Context
import android.net.Uri

/**
 * 扫码能力扩展点（spec 5.1）。
 *
 * 默认实现基于 ZXing；未来可替换为其他实现而不改领域层。
 * 相机权限由调用方（PairActivity）按需申请。
 */
interface QrScanProvider {
    /** 从相册图片解码二维码文本，失败返回 null。 */
    fun decodeFromImage(uri: Uri): String?
}

class ZxingQrScanProvider(private val context: Context) : QrScanProvider {
    override fun decodeFromImage(uri: Uri): String? = ImageQrDecoder.decode(context, uri)
}
