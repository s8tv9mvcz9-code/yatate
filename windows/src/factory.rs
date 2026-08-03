//! クラスファクトリ — COM が矢立を「作る」ための窓口。
//!
//! ```text
//!   アプリが文字を打てる状態になる
//!     → TSF が CoCreateInstance(CLSID_矢立, IID_ITfTextInputProcessor)
//!       → COM が DLL を読み、DllGetClassObject を呼ぶ
//!         → ここ（Factory）が返る
//!           → Factory::CreateInstance が Tip を一つ作つて渡す
//! ```
//!
//! 作られた [`Tip`](crate::tip::Tip) は**スレッドごとに一つ**。TIP は STA で動くので、
//! 別スレッドで文字を打てば別の Tip が作られる（状態は混ざらない）。

use std::ffi::c_void;

use windows::core::{implement, Interface, Result, BOOL, GUID};
use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_INVALIDARG, E_NOINTERFACE};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};

use crate::dll::ModuleLock;
use crate::tip::Tip;

/// 矢立のクラスファクトリ。
#[implement(IClassFactory)]
pub struct Factory;

impl IClassFactory_Impl for Factory_Impl {
    /// 実体を一つ作つて、求められたインタフェースで返す。
    ///
    /// 生ポインタを受けるが `unsafe fn` にはできない——**署名は COM の
    /// インタフェース定義が決めてゐる**ため（`IClassFactory_Impl` を満たす必要がある）。
    /// 妥当性は COM の契約が保証し、こちらは null だけ自前で弾く。
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn CreateInstance(
        &self,
        punkouter: windows::core::Ref<'_, windows::core::IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> Result<()> {
        if ppvobject.is_null() {
            return Err(E_INVALIDARG.into());
        }
        // SAFETY: null でないことを確かめた出力先。
        unsafe { *ppvobject = std::ptr::null_mut() };

        // 集約（aggregation）は使はない。COM の作法として明示的に断る。
        if punkouter.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }

        let tip: windows::core::IUnknown = Tip::new().into();
        // SAFETY: riid は COM が寄越す有効な GUID、ppvobject は上で null 初期化済み。
        unsafe {
            let hr = tip.query(&*riid, ppvobject);
            if hr.is_err() {
                return Err(E_NOINTERFACE.into());
            }
        }
        Ok(())
    }

    /// DLL を落とさせないための錠。COM が「まだ使ふ」と言つてゐる間は保つ。
    fn LockServer(&self, flock: BOOL) -> Result<()> {
        if flock.as_bool() {
            ModuleLock::acquire();
        } else {
            ModuleLock::release();
        }
        Ok(())
    }
}

impl Factory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}
