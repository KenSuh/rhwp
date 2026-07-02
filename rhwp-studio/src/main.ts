// 스탠드얼론 앱 진입점 — 초기화 로직은 bootstrap.ts 로 추출 (Task #1790 1단계).
import { bootstrap } from './bootstrap';

bootstrap(document.getElementById('studio-root') as HTMLElement);
