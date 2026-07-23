# MCP ツール実装・doc comment 規約

## MCP ツール実装パターン

全ての MCP ツールハンドラー（`src/handlers/*.rs`）は以下のパターンに従う:

1. Mutex をロック: `let mut data = self.data.lock().unwrap();`
2. `GtdData` に対する操作を実行
3. ロックを解放: `drop(data);`
4. ディスクに保存: `self.save_data()?`（コミットメッセージを変えたい場合は `save_data_with_message`）
5. エラーには `bail!()` を使用（`mcp_attr::bail`）

```rust
pub async fn handle_inbox(&self, id: String, title: String, ...) -> McpResult<String> {
    let mut data = self.data.lock().unwrap();
    data.add_nota(nota);
    drop(data);
    if let Err(e) = self.save_data() {
        bail!("Failed to save: {}", e);
    }
    Ok(format!("Created: {}", id))
}
```

保存前に必ずロックを解放すること（`save_data` は内部で再ロックするため、解放しないとデッドロックする）。

## doc comment の公開範囲

**重要**: `#[tool]` と `#[mcp_server]` の doc comment は MCP クライアント（Claude Desktop などの LLM）に直接公開され、ツールの使い方を理解するための**唯一の情報源**となる。

- **`#[tool]` 関数の doc comment**: ツールの説明として表示
- **`#[mcp_server]` impl の doc comment**: サーバー全体の説明として表示
- **関数引数の doc comment**: 各パラメータの説明として表示
- これらは LLM のトークン消費量に直接影響する

## doc comment 記述ルール

1. **関数の説明は関数名に沿った動詞を使用**
   - 例: `empty_trash` には "Purge/Delete" 系、関数名と説明の動詞を一致させ、MCP クライアントの理解を容易にする

2. **`Option<T>` 引数には必ず "Optional" を明記**
   - 悪い例: `/// New title for the task`
   - 良い例: `/// Optional: New title`
   - あわせて `""`（空文字列）でクリアできることを明記する（update 系）

3. **ID 命名を誘導する**
   - Nota ID は kebab-case・動詞始まり・簡潔（3-5語）・プロジェクトプレフィックス推奨を doc comment 内で案内（例: `"fix-io-button"`, `"eci-fix-button"`）
   - **ID は不変**であることを明記（"IDs are immutable - choose carefully"）
   - project 参照には意味のある略称を推奨（悪い例: `"project-1"` / 良い例: `"website-redesign"`, `"q1-budget"`）
   - バッチ引数（`ids`）には配列形式の例を記載。レガシーなプレーン数字形式は記載しない（内部で自動修正されるが、クライアントには推奨形式のみ提示）

4. **GTD ワークフローのコンテキストを含める**
   - 単なる機能説明でなく、GTD 手法の中で**いつ・なぜ**使うかを説明
   - パターン: `**Capture**: ...` / `**When**: ...` / `**Next**: ...` / `**Tip**: ...` の太字ラベル形式

5. **トークン消費を最小化**
   - 簡潔・明確に。自明な情報は省略
   - 記号（`→`, `|`, `/`）を活用して情報密度を高める
   - 例: `inbox(start) | next_action(ready) | waiting_for(blocked) | ...`

6. **ステータス列挙は完全に記載**
   - status 系引数には有効値を列挙: `inbox | next_action | waiting_for | later | calendar | someday | done | reference | project | context | trash`
   - `calendar` には `start_date` 必須であることを明記

---
_created: 2026-07-23 (copilot-instructions.md からの移行を含む)_
