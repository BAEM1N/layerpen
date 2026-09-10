export const widthPresets = [2, 4, 8, 16, 24];
export const minWidth = 1, maxWidth = 64;
const names = ['아주 가늘게', '가늘게', '보통', '굵게', '아주 굵게'];

export function nextWidth(value, direction) {
  if (direction > 0) return widthPresets.find(width => width > value) ?? Math.min(maxWidth, value + 1);
  if (value > widthPresets.at(-1)) return Math.max(widthPresets.at(-1), value - 1);
  return [...widthPresets].reverse().find(width => width < value) ?? minWidth;
}

export function widthControls(name, value, label = '굵기') {
  return `<div class="width-controls" data-width-control="${name}"><div class="size-choices" data-size-group="${name}">${widthPresets.map((width, i) => `<button type="button" class="size-choice" data-size="${width}" title="${names[i]} (${width} px)" aria-label="${names[i]} (${width} px)" aria-pressed="${value === width}"><span style="--dot:${Math.max(3, width * .65)}px"></span><small>${width}</small></button>`).join('')}</div><div class="custom-width"><input type="range" data-width-range min="${minWidth}" max="${maxWidth}" step="1" value="${value}" aria-label="${label}" title="굵기 직접 조절"><label class="width-number"><input type="number" data-width-number min="${minWidth}" max="${maxWidth}" step="any" value="${value}" aria-label="${label}" title="굵기 직접 입력"><span>px</span></label></div></div>`;
}

export function syncWidthControls(group, value, force = false) {
  if (!group) return;
  // A session broadcast must not replace a number the user is still typing.
  if (!force && group.contains(document.activeElement) && document.activeElement.matches('input')) return;
  group.querySelectorAll('[data-size]').forEach(button => {
    const selected = Number(button.dataset.size) === value;
    button.classList.toggle('selected', selected);
    button.setAttribute('aria-pressed', String(selected));
  });
  group.classList.toggle('custom-selected', !widthPresets.includes(value));
  group.querySelector('[data-width-range]').value = value;
  group.querySelector('[data-width-number]').value = value;
}

export function bindWidthControls(group, getValue, apply) {
  const range = group.querySelector('[data-width-range]'), number = group.querySelector('[data-width-number]');
  let queue = Promise.resolve();
  const commit = raw => {
    if (String(raw).trim() === '' || !Number.isFinite(Number(raw))) {
      syncWidthControls(group, getValue(), true);
      return;
    }
    const value = Math.max(minWidth, Math.min(maxWidth, Number(raw)));
    syncWidthControls(group, value, true);
    queue = queue.catch(() => {}).then(() => apply(value)).finally(() => syncWidthControls(group, getValue()));
  };
  group.querySelectorAll('[data-size]').forEach(button => button.onclick = () => commit(button.dataset.size));
  range.oninput = () => syncWidthControls(group, Number(range.value), true);
  range.onchange = () => commit(range.value);
  number.onchange = () => commit(number.value);
  number.onkeydown = event => {
    if (event.key === 'Enter') { event.preventDefault(); event.stopPropagation(); commit(number.value); number.blur(); }
    if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); syncWidthControls(group, getValue(), true); number.blur(); }
  };
  for (const input of [range, number]) input.addEventListener('blur', () => queue.finally(() => syncWidthControls(group, getValue())));
}
