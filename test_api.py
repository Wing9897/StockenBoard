#!/usr/bin/env python3
"""
StockenBoard API 測試腳本
"""
import requests
import json
from datetime import datetime

API_BASE = "http://localhost:8080/api"

def test_status():
    """測試系統狀態"""
    print("\n=== 測試系統狀態 ===")
    try:
        r = requests.get(f"{API_BASE}/status", timeout=5)
        print(f"狀態碼: {r.status_code}")
        if r.status_code == 200:
            data = r.json()
            print(json.dumps(data, indent=2, ensure_ascii=False))
        else:
            print(f"錯誤: {r.text}")
    except Exception as e:
        print(f"連接失敗: {e}")

def test_prices():
    """測試獲取價格"""
    print("\n=== 測試獲取所有價格 ===")
    try:
        r = requests.get(f"{API_BASE}/prices", timeout=5)
        print(f"狀態碼: {r.status_code}")
        if r.status_code == 200:
            data = r.json()
            print(f"總數: {data['count']}")
            if data['prices']:
                print(f"第一筆: {json.dumps(data['prices'][0], indent=2, ensure_ascii=False)}")
        else:
            print(f"錯誤: {r.text}")
    except Exception as e:
        print(f"連接失敗: {e}")

def test_subscriptions():
    """測試獲取訂閱"""
    print("\n=== 測試獲取訂閱列表 ===")
    try:
        r = requests.get(f"{API_BASE}/subscriptions", timeout=5)
        print(f"狀態碼: {r.status_code}")
        if r.status_code == 200:
            data = r.json()
            print(f"總數: {data['count']}")
            if data['subscriptions']:
                for sub in data['subscriptions'][:3]:
                    print(f"  - {sub['symbol']} ({sub['provider']}) [{sub['sub_type']}]")
        else:
            print(f"錯誤: {r.text}")
    except Exception as e:
        print(f"連接失敗: {e}")

def test_history():
    """測試獲取歷史"""
    print("\n=== 測試獲取歷史數據 ===")
    try:
        # 獲取最近 1 小時的數據
        now = int(datetime.now().timestamp())
        from_ts = now - 3600
        r = requests.get(
            f"{API_BASE}/history",
            params={"from": from_ts, "to": now, "limit": 10},
            timeout=5
        )
        print(f"狀態碼: {r.status_code}")
        if r.status_code == 200:
            data = r.json()
            print(f"總數: {data['count']}")
            if data['records']:
                print(f"第一筆: {json.dumps(data['records'][0], indent=2, ensure_ascii=False)}")
        else:
            print(f"錯誤: {r.text}")
    except Exception as e:
        print(f"連接失敗: {e}")

if __name__ == "__main__":
    print("StockenBoard API 測試")
    print("=" * 50)
    print("請確保 StockenBoard 正在運行")
    print("API 地址: http://localhost:8080")
    print("=" * 50)
    
    test_status()
    test_subscriptions()
    test_prices()
    test_history()
    
    print("\n測試完成！")
