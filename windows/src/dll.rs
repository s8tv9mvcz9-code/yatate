//! DLL の四つの出口 — COM が矢立を見つけ、読み、外すための約束。
//!
//! TIP は **文字を受け取る全アプリのプロセスへ読み込まれる in-proc COM DLL** である。
//! メモ帳にも Word にも Chrome にも入るので、ここでの作法違反は
//! 「特定のアプリだけ落ちる」といふ形で現れる。
//!
//! | 出口 | 誰が呼ぶ | 何を答へる |
//! |---|---|---|
//! | `DllGetClassObject` | COM（`CoCreateInstance` の途中） | クラスファクトリ |
//! | `DllCanUnloadNow` | COM（暇なとき） | 「外してよいか」 |
//! | `DllRegisterServer` | `regsvr32`（管理者） | 登録する |
//! | `DllUnregisterServer` | `regsvr32 /u` | 登録を外す |
//!
//! これらは **`#[no_mangle]` かつ `extern "system"`** でなければならない。
//! 名前が飾られると COM から見えず、呼出規約が違ふと沈黙して壊れる——
//! どちらも「読み込まれない DLL」といふ同じ症状になり、原因が見え難い。
//! だから [`crate::dll`] の出口は **エクスポート表の検査**で機械的に守る
//! （`windows/README.md` の「Windows 機なしで守れるところ」）。

use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};

use windows::core::{Interface, BOOL, GUID, HRESULT};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, E_FAIL, E_INVALIDARG, HINSTANCE, S_FALSE, S_OK,
};
use windows::Win32::System::Com::IClassFactory;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

use crate::factory::Factory;
use crate::registration;

/// この DLL の実体（`DllMain` で受け取り、登録のとき道を引くのに使ふ）。
static MODULE_HANDLE: AtomicIsize = AtomicIsize::new(0);

/// 使用中の数。0 でなければ COM は DLL を外せない。
static LOCK_COUNT: AtomicU32 = AtomicU32::new(0);

/// 参照計数の出入口（[`crate::factory`] の `LockServer` と実体の生存が使ふ）。
pub struct ModuleLock;

impl ModuleLock {
    pub fn acquire() {
        LOCK_COUNT.fetch_add(1, Ordering::AcqRel);
    }

    pub fn release() {
        // 0 を下回らせない（下回ると「まだ使つてゐるのに外される」になる）
        let _ = LOCK_COUNT.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
            Some(n.saturating_sub(1))
        });
    }

    pub fn count() -> u32 {
        LOCK_COUNT.load(Ordering::Acquire)
    }
}

/// DLL の実体。
pub fn module_handle() -> isize {
    MODULE_HANDLE.load(Ordering::Acquire)
}

/// COM が DLL を読んだ・外した瞬間。
///
/// # Safety
/// COM ローダだけが呼ぶ。ここで重い仕事をしてはならない（ローダロックの中である）。
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    hinstance: HINSTANCE,
    reason: u32,
    _reserved: *mut c_void,
) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        MODULE_HANDLE.store(hinstance.0 as isize, Ordering::Release);
    }
    true.into()
}

/// クラスファクトリを渡す。**矢立の CLSID 以外は断る。**
///
/// # Safety
/// COM だけが呼ぶ。`rclsid` / `riid` / `ppv` はいづれも COM が用意した有効な領域。
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() || rclsid.is_null() || riid.is_null() {
        return E_INVALIDARG;
    }
    // SAFETY: 上で null を除いた。
    unsafe {
        *ppv = std::ptr::null_mut();

        if *rclsid != registration::clsid() {
            return CLASS_E_CLASSNOTAVAILABLE;
        }

        let factory: IClassFactory = Factory::new().into();
        factory.query(&*riid, ppv)
    }
}

/// 外してよいか。使用中が 0 なら `S_OK`（外してよい）、さもなくば `S_FALSE`。
///
/// # Safety
/// COM だけが呼ぶ。
#[no_mangle]
pub unsafe extern "system" fn DllCanUnloadNow() -> HRESULT {
    if ModuleLock::count() == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

/// 登録（`regsvr32 yatate_windows.dll`。**管理者権限が要る**）。
///
/// # Safety
/// `regsvr32` だけが呼ぶ。
#[no_mangle]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    match registration::register_server(module_handle()) {
        Ok(()) => S_OK,
        Err(e) => {
            // 途中まで書いた登録を残さない（半端な登録は次回の登録も壊す）
            let _ = registration::unregister_server();
            if e.code().is_err() {
                e.code()
            } else {
                E_FAIL
            }
        }
    }
}

/// 登録の取り消し（`regsvr32 /u yatate_windows.dll`）。
///
/// # Safety
/// `regsvr32` だけが呼ぶ。
#[no_mangle]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    match registration::unregister_server() {
        Ok(()) => S_OK,
        Err(e) => {
            if e.code().is_err() {
                e.code()
            } else {
                E_FAIL
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 錠は増減し_零を下回らない() {
        let base = ModuleLock::count();
        ModuleLock::acquire();
        assert_eq!(ModuleLock::count(), base + 1);
        ModuleLock::release();
        assert_eq!(ModuleLock::count(), base);
        // 余分に外しても 0 未満にならない（飽和させる）
        ModuleLock::release();
        ModuleLock::release();
        assert_eq!(
            ModuleLock::count(),
            0,
            "使用中の数が負に回り込んではならない"
        );
    }
}
