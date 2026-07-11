// rAF 合帧的批量收集器: 高频事件流(git 逐行输出)先入普通数组, 每帧至多一次
// 刷入 $state——避免每行一次细粒度更新 + 自动滚底强制布局(大输出时逐行渲染卡整个窗口)

/** 批量收集器句柄 */
export interface Batcher<T> {
  /** 收一条(本帧首条时预约 rAF 刷新) */
  push: (item: T) => void;
  /** 立即刷出未落地条目(结局尾行写入前调用, 保证顺序) */
  drain: () => void;
}

/** 创建收集器; flush 在 rAF 或 drain 时收到该帧累积的批次 */
export function rafBatcher<T>(flush: (batch: T[]) => void): Batcher<T> {
  let buf: T[] = [];
  let pending = false;
  const drain = () => {
    pending = false;
    if (buf.length) {
      const b = buf;
      buf = [];
      flush(b);
    }
  };
  return {
    push(item: T) {
      buf.push(item);
      if (!pending) {
        pending = true;
        requestAnimationFrame(drain);
      }
    },
    drain,
  };
}
