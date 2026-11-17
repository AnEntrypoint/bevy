const css = `
.z{position:absolute;z-index:20}
.danc,.so-anc{z-index:-1}
.dtag{transform:translate(-50%,-50%);text-align:center;font-size:11px;letter-spacing:.14em;white-space:nowrap;color:#9ad7ff}
.ev-tag{transform:translate(-50%,-100%);text-align:center;font-size:12px;letter-spacing:.12em;white-space:nowrap;color:#ffb000}
.so-tag{transform:translate(-50%,-100%);text-align:center;font-size:11px;letter-spacing:.12em;white-space:nowrap;color:#3be8ff}
.cow{transform:translate(-50%,-50%);font-size:12px;white-space:nowrap;color:#ff3b3b}
@keyframes dzin{0%{opacity:0;transform:translateY(-4px)}15%{opacity:1;transform:none}75%{opacity:1}100%{opacity:0;visibility:hidden}}
`;

export default function () {
  return `<style>${css}</style>
<div id="dzNEXUS" class="z" style="top:24%;left:0;right:0px;margin:0 auto;width:max-content;text-align:center;font-size:15px;font-weight:700;color:#c9a0ff;animation:dzin 4s forwards">NEXUS</div>
<div class="danc" data-world-anchor="fixture-district" data-anchor-offset="0 0.4 0"><div class="dtag">DISTRICT</div></div>
<div data-world-anchor="fixture-event" data-anchor-offset="0 3 0" style="z-index:-1"><div class="ev-tag">EVENT</div></div>
<div data-world-anchor="fixture-foe" data-anchor-offset="0 1 0"><div class="cow cofoe" style="position:relative"><b>FOE</b></div></div>
<div data-world-anchor="fixture-pad" data-anchor-offset="-2 4 0"><div class="cow cobeac" style="position:relative"><b>BEACON</b></div></div>
<div class="so-anc" data-world-anchor="fixture-ping" data-anchor-offset="0 1.6 0"><div class="so-tag so-ping">PING</div></div>`;
}
