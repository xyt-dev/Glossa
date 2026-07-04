import { useState } from "react";
import type { Config, Mode, Profile } from "../types";
import { THEMES } from "../types";
import { isTauri } from "../platform";
import Dropdown from "./Dropdown";

const EFFORT_OPTIONS = [
  { value: "", label: "no thinking" },
  { value: "low", label: "low" },
  { value: "medium", label: "medium" },
  { value: "high", label: "high" },
  { value: "xhigh", label: "xhigh" },
];

interface Props {
  config: Config;
  onSave: (cfg: Config) => Promise<void>;
  onClose: () => void;
  onPreviewTheme: (theme: string) => void;
  onPreviewZoom: (zoom: number) => void;
}

const emptyProfile = (): Profile => ({
  name: "new-profile",
  base_url: "https://api.openai.com/v1",
  api_key: "",
  api_key_env: "OPENAI_API_KEY",
  model: "",
  translate_effort: null,
  chat_effort: null,
  provider: null,
  temperature: null,
  extra: null,
});

export default function Settings({
  config,
  onSave,
  onClose,
  onPreviewTheme,
  onPreviewZoom,
}: Props) {
  const [draft, setDraft] = useState<Config>(() => JSON.parse(JSON.stringify(config)));
  const [profileIdx, setProfileIdx] = useState(() =>
    Math.max(0, config.profiles.findIndex((p) => p.name === config.active_profile)),
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const profile = draft.profiles[profileIdx];

  const patch = (fn: (d: Config) => void) => {
    setDraft((d) => {
      const next = JSON.parse(JSON.stringify(d)) as Config;
      fn(next);
      return next;
    });
  };

  const patchProfile = (fn: (p: Profile) => void) =>
    patch((d) => fn(d.profiles[profileIdx]));

  // Discard: revert any previewed theme/zoom back to the saved values.
  const cancel = () => {
    onPreviewTheme(config.ui.theme);
    onPreviewZoom(config.ui.zoom);
    onClose();
  };

  const save = async () => {
    if (!draft.profiles.length) {
      setError("至少保留一个 profile");
      return;
    }
    if (!draft.profiles.some((p) => p.name === draft.active_profile)) {
      setError("active_profile 不存在，请先选择使用的 profile");
      return;
    }
    setSaving(true);
    try {
      await onSave(draft);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="modal-backdrop" onClick={cancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <h2>设置</h2>
          <button className="close-btn" onClick={cancel}>
            ×
          </button>
        </div>

        <div className="modal-body">
          <section>
            <h3>外观</h3>
            <label>
              主题
              <Dropdown
                value={draft.ui.theme}
                options={THEMES.map((t) => ({ value: t.id, label: t.label }))}
                onChange={(theme) => {
                  patch((d) => (d.ui.theme = theme));
                  onPreviewTheme(theme);
                }}
              />
            </label>
            {isTauri && (
              <label>
                界面缩放（写入 [ui] zoom，1.0 = 100%）
                <input
                  type="number"
                  step={0.1}
                  min={0.6}
                  max={2.5}
                  value={draft.ui.zoom}
                  onChange={(e) => {
                    const z = Number(e.target.value) || 1;
                    patch((d) => (d.ui.zoom = z));
                    onPreviewZoom(z);
                  }}
                />
              </label>
            )}
          </section>

          <section>
            <h3>会话</h3>
            <label>
              默认模式
              <Dropdown
                value={draft.session.default_mode}
                options={[
                  { value: "translate", label: "翻译" },
                  { value: "chat", label: "聊天" },
                ]}
                onChange={(v) => patch((d) => (d.session.default_mode = v as Mode))}
              />
            </label>
            <label>
              上下文窗口（条）
              <input
                type="number"
                min={2}
                value={draft.session.max_context_messages}
                onChange={(e) =>
                  patch((d) => (d.session.max_context_messages = Number(e.target.value) || 2))
                }
              />
            </label>
          </section>

          <section>
            <h3>选词讲解</h3>
            <label>
              IELTS band 下限
              <input
                type="number"
                step={0.5}
                min={4}
                max={9}
                value={draft.memory.min_ielts_band}
                onChange={(e) =>
                  patch((d) => (d.memory.min_ielts_band = Number(e.target.value) || 7))
                }
              />
            </label>
            <label>
              memory 词条上限
              <input
                type="number"
                min={0}
                value={draft.memory.max_context_words}
                onChange={(e) =>
                  patch((d) => (d.memory.max_context_words = Number(e.target.value) || 0))
                }
              />
            </label>
          </section>

          {isTauri && (
            <section>
              <h3>Web 服务</h3>
              <label>
                开启 Web 服务
                <Dropdown
                  value={draft.web.enabled ? "on" : "off"}
                  options={[
                    { value: "off", label: "关闭" },
                    { value: "on", label: "开启" },
                  ]}
                  onChange={(v) => patch((d) => (d.web.enabled = v === "on"))}
                />
              </label>
              <label>
                端口
                <input
                  type="number"
                  min={1}
                  max={65535}
                  value={draft.web.port}
                  onChange={(e) =>
                    patch((d) => (d.web.port = Number(e.target.value) || 8040))
                  }
                />
              </label>
              <div className="settings-hint">
                保存后立即生效；开启后局域网设备可访问 http://本机IP:{draft.web.port}/
                （也可随时用 `glossa web` 单独启动）。
              </div>
            </section>
          )}

          <section>
            <h3>API Profiles</h3>
            <div className="profile-row">
              <label>
                编辑
                <Dropdown
                  value={String(profileIdx)}
                  options={draft.profiles.map((p, i) => ({
                    value: String(i),
                    label: p.name,
                  }))}
                  onChange={(v) => setProfileIdx(Number(v))}
                />
              </label>
              <button
                onClick={() => {
                  patch((d) => d.profiles.push(emptyProfile()));
                  setProfileIdx(draft.profiles.length);
                }}
              >
                ＋ 新增
              </button>
              <button
                disabled={draft.profiles.length <= 1}
                onClick={() => {
                  patch((d) => d.profiles.splice(profileIdx, 1));
                  setProfileIdx(0);
                }}
              >
                删除
              </button>
            </div>
            {profile && (
              <div className="profile-fields">
                <label>
                  名称
                  <input
                    value={profile.name}
                    onChange={(e) => patchProfile((p) => (p.name = e.target.value))}
                  />
                </label>
                <label>
                  Base URL
                  <input
                    value={profile.base_url}
                    onChange={(e) => patchProfile((p) => (p.base_url = e.target.value))}
                  />
                </label>
                <label>
                  API Key（留空则用环境变量）
                  <input
                    type="password"
                    value={profile.api_key}
                    onChange={(e) => patchProfile((p) => (p.api_key = e.target.value))}
                  />
                </label>
                <label>
                  Key 环境变量
                  <input
                    value={profile.api_key_env}
                    onChange={(e) => patchProfile((p) => (p.api_key_env = e.target.value))}
                  />
                </label>
                <label>
                  模型
                  <input
                    value={profile.model}
                    onChange={(e) => patchProfile((p) => (p.model = e.target.value))}
                  />
                </label>
                <label>
                  Provider 兼容层
                  <Dropdown
                    value={profile.provider ?? ""}
                    options={[
                      { value: "", label: "自动（按 URL 判断）" },
                      { value: "deepseek", label: "DeepSeek" },
                      { value: "openai", label: "OpenAI 标准" },
                    ]}
                    onChange={(v) => patchProfile((p) => (p.provider = v || null))}
                  />
                </label>
                <label>
                  翻译模式思考
                  <Dropdown
                    value={profile.translate_effort ?? ""}
                    options={EFFORT_OPTIONS}
                    onChange={(v) =>
                      patchProfile((p) => (p.translate_effort = v || null))
                    }
                  />
                </label>
                <label>
                  聊天模式思考
                  <Dropdown
                    value={profile.chat_effort ?? ""}
                    options={EFFORT_OPTIONS}
                    onChange={(v) => patchProfile((p) => (p.chat_effort = v || null))}
                  />
                </label>
                <label>
                  Temperature（留空不传）
                  <input
                    type="number"
                    step={0.1}
                    value={profile.temperature ?? ""}
                    onChange={(e) =>
                      patchProfile(
                        (p) =>
                          (p.temperature =
                            e.target.value === "" ? null : Number(e.target.value)),
                      )
                    }
                  />
                </label>
              </div>
            )}
            <label>
              使用的 profile
              <Dropdown
                value={draft.active_profile}
                options={draft.profiles.map((p) => ({ value: p.name, label: p.name }))}
                onChange={(v) => patch((d) => (d.active_profile = v))}
              />
            </label>
          </section>

          {error && <div className="settings-error">{error}</div>}
        </div>

        <div className="modal-foot">
          <button onClick={cancel}>取消</button>
          <button className="primary" onClick={save} disabled={saving}>
            {saving ? "保存中…" : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}
