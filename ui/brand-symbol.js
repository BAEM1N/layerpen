// Preserve the approved B artwork; the mask only removes its white background.
// The tight viewBox keeps the nib's original proportions at toolbar size.
export const brandSymbol = `<svg class="brand-symbol" viewBox="394 195 475 860" aria-hidden="true" focusable="false">
  <defs>
    <filter id="brand-ink" color-interpolation-filters="sRGB">
      <feColorMatrix type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  -.2126 -.7152 -.0722 0 1"/>
      <feComponentTransfer><feFuncA type="linear" slope="1.02" intercept="-0.02"/></feComponentTransfer>
    </filter>
    <mask id="brand-silhouette" maskUnits="userSpaceOnUse" x="394" y="195" width="475" height="860" style="mask-type:alpha">
      <image href="assets/pointory-symbol-b.png" width="1254" height="1254" filter="url(#brand-ink)"/>
    </mask>
  </defs>
  <rect x="394" y="195" width="475" height="860" fill="currentColor" mask="url(#brand-silhouette)"/>
</svg>`;
