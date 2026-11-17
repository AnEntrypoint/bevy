const css = `
.z{position:absolute;z-index:20}
.danc,.so-anc{z-index:-1}
.dtag{transform:translate(-50%,-50%);text-align:center;font-size:11px;letter-spacing:.14em;white-space:nowrap;color:#9ad7ff}
.ev-tag{transform:translate(-50%,-100%);text-align:center;font-size:12px;letter-spacing:.12em;white-space:nowrap;color:#ffb000}
.so-tag{transform:translate(-50%,-100%);text-align:center;font-size:11px;letter-spacing:.12em;white-space:nowrap;color:#3be8ff}
.cow{transform:translate(-50%,-50%);font-size:12px;white-space:nowrap;color:#ff3b3b}
@keyframes pulse{from{opacity:1}to{opacity:.2}}
@keyframes pulsein{from{opacity:.3}to{opacity:1}}
.row{position:absolute;left:20px;font-size:13px;white-space:nowrap}
@keyframes slide{0%{transform:translateX(0px) scale(1) rotate(0deg)}50%{transform:translateX(30px) scale(1.5) rotate(45deg)}100%{transform:translateX(60px) scale(1) rotate(90deg)}}
@keyframes nudge{50%{transform:translate(20px,10px)}}
@keyframes tint{from{color:#ff0000;background-color:#0000ff}to{color:#00ff00;background-color:#ffff00}}
@keyframes sweep{from{opacity:.2}to{opacity:1}}
@keyframes blink{from{opacity:1}to{opacity:.2}}
@keyframes hue{from{color:#ff0000}to{color:#0000ff}}
@keyframes dzin{0%{opacity:0;transform:translateY(-4px)}15%{opacity:1;transform:none}75%{opacity:1}100%{opacity:0;visibility:hidden}}
`;

export default function (ctx, player) {
  const step = Number(player?.state?.step ?? 0);
  return `<style>${css}</style>
<div id="lhAnim" class="row" style="top:20px;color:#ffffff;animation-name:pulse;animation-duration:2s;animation-delay:0.5s;animation-timing-function:linear;animation-iteration-count:2;animation-fill-mode:both">LONGHAND</div>
<div id="lhPaused" class="row" style="top:44px;color:#ffffff;animation-name:pulsein;animation-duration:2s;animation-fill-mode:both;animation-play-state:paused">PAUSED</div>
<div id="trFade" class="row" style="top:68px;color:#ffd166;transition:opacity 1.5s ease-in-out;opacity:${step ? 0.25 : 1}">FADE</div>
<div id="anTransform" class="row" style="top:116px;color:#ffffff;animation:slide 4s linear forwards">SLIDE</div>
<div id="anCompose" class="row" style="top:140px;left:60px;color:#ffffff;transform:translate(-20px,0px);animation:nudge 3s ease-in-out forwards">NUDGE</div>
<div id="anColor" class="row" style="top:164px;animation:tint 3s linear forwards">TINT</div>
<div id="dirNormal" class="row" style="top:188px;color:#ffffff;animation:sweep 1s ease-in 3 normal both">NORMAL</div>
<div id="dirReverse" class="row" style="top:212px;color:#ffffff;animation:sweep 1s ease-in 3 reverse both">REVERSE</div>
<div id="dirAlternate" class="row" style="top:236px;color:#ffffff;animation:sweep 1s ease-in 3 alternate both">ALTERNATE</div>
<div id="dirAltReverse" class="row" style="top:260px;color:#ffffff;animation:sweep 1s ease-in 3 alternate-reverse both">ALTREVERSE</div>
<div id="ppMixed" class="row" style="top:284px;color:#ff0000;animation:blink 1.5s linear infinite, hue 2s linear infinite;animation-play-state:running, paused">MIXED</div>
<div id="trRev" class="row" style="top:308px;color:#ffffff;transition:opacity 2s linear;opacity:${step === 1 ? 0.2 : 1}">REVERSAL</div>
<div id="trHide" class="row" style="top:92px;color:#ff6b6b;transition-property:opacity,visibility;transition-duration:1s;transition-timing-function:linear;opacity:${step ? 0 : 1};visibility:${step ? 'hidden' : 'visible'}">HIDE</div>
<div id="dzNEXUS" class="z" style="top:24%;left:0;right:0px;margin:0 auto;width:max-content;text-align:center;font-size:15px;font-weight:700;color:#c9a0ff;animation:dzin 4s forwards">NEXUS</div>
<div class="danc" data-world-anchor="fixture-district" data-anchor-offset="0 0.4 0"><div class="dtag">DISTRICT</div></div>
<div data-world-anchor="fixture-event" data-anchor-offset="0 3 0" style="z-index:-1"><div class="ev-tag">EVENT</div></div>
<div data-world-anchor="fixture-foe" data-anchor-offset="0 1 0"><div class="cow cofoe" style="position:relative"><b>FOE</b></div></div>
<div data-world-anchor="fixture-pad" data-anchor-offset="-2 4 0"><div class="cow cobeac" style="position:relative"><b>BEACON</b></div></div>
<div class="so-anc" data-world-anchor="fixture-ping" data-anchor-offset="0 1.6 0"><div class="so-tag so-ping">PING</div></div>`;
}
