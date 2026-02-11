import Foundation
import Speech

/// Speech.framework が利用可能かチェック
@_cdecl("swift_speech_is_available")
func swiftSpeechIsAvailable() -> Bool {
    guard let recognizer = SFSpeechRecognizer() else {
        return false
    }
    return recognizer.isAvailable
}

/// WAV ファイルから音声認識を実行し、結果を JSON 文字列で返す
///
/// 戻り値: malloc で確保された C 文字列（呼び出し側で swift_free_string で解放）
/// JSON 形式: {"text": "認識結果", "confidence": 0.95}
@_cdecl("swift_speech_recognize_file")
func swiftSpeechRecognizeFile(
    filePath: UnsafePointer<CChar>,
    language: UnsafePointer<CChar>
) -> UnsafeMutablePointer<CChar> {
    let filePathStr = String(cString: filePath)
    let languageStr = String(cString: language)
    let url = URL(fileURLWithPath: filePathStr)

    let emptyResult = strdup("{\"text\":\"\",\"confidence\":0.0}")!

    guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: languageStr)) else {
        return emptyResult
    }

    guard recognizer.isAvailable else {
        return emptyResult
    }

    let request = SFSpeechURLRecognitionRequest(url: url)
    request.shouldReportPartialResults = false

    let semaphore = DispatchSemaphore(value: 0)
    var resultText = ""
    var resultConfidence: Float = 0.0

    recognizer.recognitionTask(with: request) { result, error in
        defer { semaphore.signal() }

        if let error = error {
            NSLog("[VTSwift] Speech recognition error: \(error.localizedDescription)")
            return
        }

        guard let result = result, result.isFinal else {
            return
        }

        resultText = result.bestTranscription.formattedString

        // 平均 confidence を計算
        let segments = result.bestTranscription.segments
        if !segments.isEmpty {
            resultConfidence = segments.map { $0.confidence }.reduce(0, +) / Float(segments.count)
        }
    }

    // 30秒タイムアウト
    let timeout = DispatchTime.now() + .seconds(30)
    let waitResult = semaphore.wait(timeout: timeout)

    if waitResult == .timedOut {
        NSLog("[VTSwift] Speech recognition timed out")
        return emptyResult
    }

    // JSON エスケープ
    let escapedText = resultText
        .replacingOccurrences(of: "\\", with: "\\\\")
        .replacingOccurrences(of: "\"", with: "\\\"")
        .replacingOccurrences(of: "\n", with: "\\n")
        .replacingOccurrences(of: "\r", with: "\\r")
        .replacingOccurrences(of: "\t", with: "\\t")

    let json = "{\"text\":\"\(escapedText)\",\"confidence\":\(resultConfidence)}"
    return strdup(json)!
}

/// Rust 側から呼ばれる文字列解放関数
@_cdecl("swift_free_string")
func swiftFreeString(ptr: UnsafeMutablePointer<CChar>) {
    free(ptr)
}
