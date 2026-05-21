import { Activity, BookOpenText, Gauge, KeyRound, Route, Server, SlidersHorizontal } from "lucide-react";

export type Tab = "overview" | "upstreams" | "models" | "keys" | "logs" | "settings" | "tutorial";

export const navItems = [
  { id: "overview" as const, icon: <Gauge size={18} />, label: "总览", path: "/admin/overview" },
  { id: "upstreams" as const, icon: <Server size={18} />, label: "渠道", path: "/admin/upstreams" },
  { id: "models" as const, icon: <Route size={18} />, label: "模型映射", path: "/admin/models" },
  { id: "keys" as const, icon: <KeyRound size={18} />, label: "密钥", path: "/admin/keys" },
  { id: "logs" as const, icon: <Activity size={18} />, label: "日志", path: "/admin/logs" },
  { id: "settings" as const, icon: <SlidersHorizontal size={18} />, label: "设置", path: "/admin/settings" },
  { id: "tutorial" as const, icon: <BookOpenText size={18} />, label: "教程", path: "/admin/tutorial" }
];

export function tabTitle(tab: Tab) {
  return { overview: "运行总览", upstreams: "渠道管理", models: "模型映射", keys: "分发密钥", logs: "请求日志", settings: "运行设置", tutorial: "使用教程" }[tab];
}

export function tabSubtitle(tab: Tab) {
  return {
    overview: "",
    upstreams: "",
    models: "",
    keys: "",
    logs: "",
    settings: "",
    tutorial: ""
  }[tab];
}
