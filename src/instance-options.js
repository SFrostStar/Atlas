'use strict';
(function(root){
  const colors=Object.freeze({amber:'#ff8a3d',mint:'#86b3a2',blue:'#94a8db',violet:'#ba9ccc',rose:'#f28cb0'});
  const icons=Object.freeze(['box','mountain','trees','flame','snowflake','sword','pickaxe','gem','rocket','moon','sun','star','castle','compass','map','zap','leaf','skull','gamepad-2','heart']);
  const api=Object.freeze({colors,icons});
  if(typeof module!=='undefined')module.exports=api;else root.AtlasInstanceOptions=api;
})(typeof window==='undefined'?globalThis:window);
