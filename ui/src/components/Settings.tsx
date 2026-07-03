import { useState } from "react";
import type { Config, Mode, Profile } from "../types";
import { THEMES } from "../types";

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
  effort: null,
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
              <select
                value={draft.ui.theme}
                onChange={(e) => {
                  const theme = e.target.value;
                  patch((d) => (d.ui.theme = theme));
                  onPreviewTheme(theme);
                }}
              >
                {THEMES.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.label}
                  </option>
                ))}
              </select>
            </label>
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
          </section>

          <section>
            <h3>会话</h3>
            <label>
              默认模式
              <select
                value={draft.session.default_mode}
                onChange={(e) =>
                  patch((d) => (d.session.default_mode = e.target.value as Mode))
                }
              >
                <option value="translate">严格翻译</option>
                <option value="chat">聊天</option>
              </select>
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

          <section>
            <h3>API Profiles</h3>
            <div className="profile-row">
              <label>
                编辑
                <select
                  value={profileIdx}
                  onChange={(e) => setProfileIdx(Number(e.target.value))}
                >
                  {draft.profiles.map((p, i) => (
                    <option key={i} value={i}>
                      {p.name}
                    </option>
                  ))}
                </select>
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
                  Effort（reasoning_effort，留空不传）
                  <input
                    value={profile.effort ?? ""}
                    placeholder="low / medium / high"
                    onChange={(e) =>
                      patchProfile((p) => (p.effort = e.target.value.trim() || null))
                    }
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
              <select
                value={draft.active_profile}
                onChange={(e) => patch((d) => (d.active_profile = e.target.value))}
              >
                {draft.profiles.map((p, i) => (
                  <option key={i} value={p.name}>
                    {p.name}
                  </option>
                ))}
              </select>
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
