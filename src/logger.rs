use std::io::Write;

// set RUST_LOG=DEBUG to see debug logs
/// ロガーを初期化するよ！٩(ˊᗜˋ*)و
/// 環境変数 `RUST_LOG` (例: `DEBUG`, `INFO`) でログレベルをコントロールできるんだ♪
pub fn init() {
    env_logger::Builder::from_default_env()
        // タイムスタンプをミリ秒まで表示する設定だよ！
        .format_timestamp_millis()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} <{}> {}",
                buf.timestamp(),
                record.level(),
                record.args()
            )
        })
        // これでロガーが動き出すよ！
        .init();
}

/// デバッグレベルのメッセージをログに出力するよ！(<em>´ω｀</em>)
pub fn log_debug(msg: &str) {
    log::debug!("{}", msg);
}

/// 情報レベルのメッセージをログに出力するよ！(・∀・)ｲｲﾈ!!
pub fn log_info(msg: &str) {
    log::info!("{}", msg);
}

/// 警告レベルのメッセージをログに出力するよ！Σ(ﾟДﾟ；)ｱﾗﾏｯ
pub fn log_warn(msg: &str) {
    log::warn!("{}", msg);
}

/// エラーレベルのメッセージをログに出力するよ！(´；ω；｀)ｳｯ…
pub fn log_error(msg: &str) {
    log::error!("{}", msg);
}

#[cfg(test)]
mod tests {
    use super::*; // logger.rs の中身をぜーんぶ使えるようにするおまじない！

    // テストの時だけ特別にロガーを初期化する関数だよ！
    // 何回も呼ばれても大丈夫なように、`std::sync::Once` を使うんだ♪
    fn ensure_logger_initialized() {
        static LOGGER_INIT: std::sync::Once = std::sync::Once::new();
        LOGGER_INIT.call_once(|| {
            init();
        });
    }

    #[test]
    fn test_log_functions_do_not_panic() {
        // まずはロガーを初期化！ (何回呼んでも大丈夫なようにしてあるよ！)
        ensure_logger_initialized();

        // それぞれのログ関数を呼んでみて、パニックしないか確認するよ！
        // 実際にログが出るかは、ここではチェックしないけど、エラーにならないのが大事！(๑•̀ㅂ•́)و✧
        log_debug("テストだよっ！ (デバッグ)");
        log_info("テストだよっ！ (情報)");
        log_warn("テストだよっ！ (警告)");
        log_error("テストだよっ！ (エラー)");
        assert!(true, "ログ関数がエラーなく実行できたよ！やったね！");
    }
}
