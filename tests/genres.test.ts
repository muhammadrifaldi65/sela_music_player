import assert from 'node:assert/strict';
import {balanceTracks} from '../src/genreTracks.ts';
const track=(id:string)=>({id,title:id,channel:null});
assert.deepEqual(balanceTracks([[track('a'),track('b'),track('c')],[track('a'),track('d')]],4).map(t=>t.id),['a','d','b','c']);
assert.deepEqual(balanceTracks([[],[track('a'),track('b')]],1).map(t=>t.id),['a']);
assert.deepEqual(balanceTracks([],24),[]);
console.log('Genre discovery balancing regressions passed');
