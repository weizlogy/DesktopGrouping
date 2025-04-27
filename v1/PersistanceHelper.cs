using Desktop_Grouping.Groupx;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Unicode;
using System.Threading.Tasks;

namespace Desktop_Grouping {
  /// <summary>
  /// 永続化クラス
  /// </summary>
  public class PersistanceHelper {

    /// <summary>
    /// 各Groupの設定ファイルの接頭詞
    /// </summary>
    public const string PREFIX = "dg_";

    /// <summary>
    /// 設定ファイルに保存
    /// </summary>
    /// <param name="obj"></param>
    /// <returns></returns>
    public async Task Serialize(Group obj) {
      var json = JsonSerializer.Serialize(obj, new JsonSerializerOptions() {
        Encoder = JavaScriptEncoder.Create(UnicodeRanges.All),
        WriteIndented = true,
      });
      using (StreamWriter sw = new StreamWriter(PREFIX + obj.GroupID)) {
        await sw.WriteAsync(json);
      }
    }

    /// <summary>
    /// 設定ファイルから読み込み
    /// </summary>
    /// <param name="groupID"></param>
    /// <returns></returns>
    /// <exception cref="PersistanceErrorException"></exception>
    public Group? Deserialize(string groupID) {
      var fileName = PREFIX + groupID;
      if (!File.Exists(fileName)) {
        return null;
      }
      var json = File.ReadAllText(fileName);
      if (json == null) {
        throw new PersistanceErrorException(PersistanceErrorException.State.ReadFile);
      }
      var group = JsonSerializer.Deserialize<Group>(json);
      if (group == null) {
        throw new PersistanceErrorException(PersistanceErrorException.State.Deserialize);
      }

      return group;
    }

    /// <summary>
    /// 設定ファイル削除
    /// </summary>
    /// <param name="groupID"></param>
    public void Delete(string groupID) {
      File.Delete(PREFIX + groupID);
    }
  }

  /// <summary>
  /// 永続化に失敗したときの例外
  /// </summary>
  public class PersistanceErrorException : Exception {

    /// <summary>
    /// 永続化に失敗したときの状態
    /// </summary>
    public State state = State.Unknown;

    /// <summary>
    /// InnerException（もしあれば
    /// </summary>
    public Exception? BaseException { get; } = null;

    /// <summary>
    /// コンストラクター
    /// </summary>
    /// <param name="state"></param>
    public PersistanceErrorException(State state) {
      this.state = state;
    }

    /// <summary>
    /// コンストラクター
    /// </summary>
    /// <param name="state"></param>
    /// <param name="ex"></param>
    public PersistanceErrorException(State state, Exception ex) {
      this.state = state;
      this.BaseException = ex;
    }

    /// <summary>
    /// 永続化失敗状態列挙子
    /// </summary>
    public enum State {
      Unknown,
      ReadFile,
      Deserialize,
    }
  }
}
