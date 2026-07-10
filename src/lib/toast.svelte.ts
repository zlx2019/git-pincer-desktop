// 轻量 toast: 错误与提示的统一出口
/** 当前可见的 toast 列表 */
export const toasts = $state({ list: [] as { id: number; msg: string }[] });

let nextId = 1;

/** 弹出一条 5 秒后自动消失的提示 */
export function toast(msg: string) {
  const id = nextId++;
  toasts.list.push({ id, msg });
  setTimeout(() => {
    const i = toasts.list.findIndex((t) => t.id === id);
    if (i >= 0) toasts.list.splice(i, 1);
  }, 5000);
}
