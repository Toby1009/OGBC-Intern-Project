# Token ID 計算問題分析

## 測試結果
運行測試後確認：我們的計算方式與 Gemini API 不匹配。

### 輸入數據
- Oracle: `0x157Ce2d672854c848c9b79C49a8Cc6cc89176a49`
- Question ID: `0x6a0d290c8ce1536fba41988277acb17f5ee59df82f0ce52c4565c02e37bc4d09`
- Outcome Slot Count: `2`

### 結果對比
- **期望的 Condition ID** (來自 Gemini API): `0xa6468d69ef786a8ae325f9a7bda944fbea3984f3d8c6617ca321c804961999f9`
- **我們計算的 Condition ID**: `0x84265b449289fe2d463eeaaa0e777ee8d34450e7e4e9f8e9265c81206f5426f4`
- **結果**: ❌ 不匹配

## 可能的原因

### 1. Polymarket 使用自定義的計算方式
Polymarket 可能不是使用標準的 CTF (Conditional Token Framework) 計算方式。

### 2. 需要從鏈上事件直接讀取
正確的做法可能是：
1. 使用 Gemini API 提供的 `conditionId` 
2. 直接掃描鏈上的 `ConditionPreparation` 事件來獲取 token IDs
3. 不要嘗試自己計算 condition ID

### 3. 可能的解決方案

#### 方案 A：直接使用 Gemini API 的數據
- 從 Gemini API 獲取 `conditionId` 和 `clobTokenIds`
- 不需要自己計算

#### 方案 B：掃描鏈上事件
- 使用 `conditionId` 作為輸入
- 掃描 `ConditionPreparation` 事件
- 從事件中直接讀取 token IDs（而不是計算）

## 下一步
需要確認：Polymarket 的 CTF 合約在 `ConditionPreparation` 事件中是否直接包含 token IDs？
或者我們需要查看 Polymarket 的合約代碼來了解正確的計算方式。
