import assert from 'node:assert/strict';
import {english,errorMessages,resolveLocale,translate} from '../src/translations.ts';
assert.equal(resolveLocale('system','id-ID'),'id');
assert.equal(resolveLocale('system','en-GB'),'en');
assert.equal(resolveLocale('system','fr-FR'),'en');
assert.equal(resolveLocale('en','id-ID'),'en');
assert.equal(resolveLocale('id','en-US'),'id');
assert.equal(translate('en','Putar {title}',{title:'Lagu {x}'}),'Play Lagu {x}');
assert.equal(translate('id','Putar {title}',{title:'Song'}),'Putar Song');
assert.equal(translate('en','External service diagnostic'),'External service diagnostic');
for(const [key,value] of Object.entries(english)){
 assert.ok(value.trim(),`Empty translation: ${key}`);
 assert.deepEqual([...key.matchAll(/\{\w+\}/g)].map(match=>match[0]).sort(),[...value.matchAll(/\{\w+\}/g)].map(match=>match[0]).sort(),`Placeholder mismatch: ${key}`);
 assert.equal(translate('en',key),value);
}
for(const [key,[id,en]] of Object.entries(errorMessages)){
 assert.ok(id.trim()&&en.trim(),`Empty error: ${key}`);
 assert.equal(translate('id',key),id);
 assert.equal(translate('en',key),en);
}
console.log('Translation locale, interpolation, dictionary and native error contracts passed');
