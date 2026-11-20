/* Copyright 2026 Zyvor AI Labs · https://zyvor.dev
 * SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0 */
import { describe,it,expect } from 'vitest';
import { assessmentPath,operationQuery,parseAssessment,SAMPLES,ASSESSMENT_KINDS } from '../enterprise.js';
describe('enterprise contracts',()=>{
  it('allows only supported assessment paths',()=>{expect(assessmentPath('blueprint')).toBe('/api/v1/enterprise/blueprints/validate');expect(assessmentPath('gpu')).toBe('/api/v1/enterprise/assess/gpu');expect(()=>assessmentPath('../delete')).toThrow();});
  it('requires one explicit namespace',()=>{expect(operationQuery('tenant-a')).toBe('namespace=tenant-a');for(const ns of ['all','','../admin','UPPER','a&namespace=all']) expect(()=>operationQuery(ns)).toThrow();});
  it('rejects scalar and malformed evidence',()=>{expect(()=>parseAssessment('null')).toThrow();expect(()=>parseAssessment('true')).toThrow();expect(()=>parseAssessment('{')).toThrow();expect(parseAssessment('[]')).toEqual([]);});
  it('provides serializable examples for every assessment',()=>{for(const kind of ASSESSMENT_KINDS)expect(parseAssessment(JSON.stringify(SAMPLES[kind]))).toEqual(SAMPLES[kind]);});
});
