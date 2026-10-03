package com.phonetyper.wentai.domain.model

/** 配对失败原因，用于区分界面提示。 */
enum class PairingError {
    /** 二维码不是 ptyper://connect 配对码。 */
    InvalidFormat,

    /** 缺少 ip 或 pin。 */
    Incomplete,

    /** 手动录入参数不合法。 */
    InvalidManual,

    /** 电脑端不可达。 */
    ServerUnreachable,

    /** 电脑端响应非 JSON。 */
    InvalidResponse,
}

/** 配对结果。失败时不改动现有配置。 */
sealed interface PairingResult {
    data class Success(val info: PairInfo) : PairingResult
    data class Failure(val error: PairingError) : PairingResult
}
