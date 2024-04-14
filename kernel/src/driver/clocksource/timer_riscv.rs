use core::sync::atomic::{fence, Ordering};

use alloc::{string::ToString, sync::Arc};
use bitmap::{traits::BitMapOps, StaticBitmap};
use system_error::SystemError;

use crate::{
    arch::{interrupt::TrapFrame, time::riscv_time_base_freq, CurrentIrqArch, CurrentTimeArch},
    driver::base::device::DeviceId,
    exception::{
        irqdata::{IrqHandlerData, IrqLineStatus},
        irqdesc::{
            irq_desc_manager, IrqDesc, IrqFlowHandler, IrqHandleFlags, IrqHandler, IrqReturn,
        },
        manage::irq_manager,
        InterruptArch, IrqNumber,
    },
    libs::spinlock::SpinLock,
    mm::percpu::PerCpu,
    process::ProcessManager,
    smp::core::smp_get_processor_id,
    time::TimeArch,
};

pub struct RiscVSbiTimer;

static SBI_TIMER_INIT_BMP: SpinLock<StaticBitmap<{ PerCpu::MAX_CPU_NUM as usize }>> =
    SpinLock::new(StaticBitmap::new());

static mut INTERVAL_CNT: usize = 0;

impl RiscVSbiTimer {
    pub const TIMER_IRQ: IrqNumber = IrqNumber::from(5);

    fn handle_irq(trap_frame: &mut TrapFrame) -> Result<(), SystemError> {
        // 更新下一次中断时间
        kdebug!(
            "riscv_sbi_timer: handle_irq: cpu_id: {}, time: {}",
            smp_get_processor_id().data(),
            CurrentTimeArch::get_cycles() as u64
        );
        sbi_rt::set_timer(CurrentTimeArch::get_cycles() as u64 + unsafe { INTERVAL_CNT } as u64);
        ProcessManager::update_process_times(trap_frame.is_from_user());
        Ok(())
    }

    fn enable() {
        unsafe { riscv::register::sie::set_stimer() };
    }

    fn disable() {
        unsafe { riscv::register::sie::clear_stimer() };
    }
}

/// riscv 初始化本地调度时钟源
#[inline(never)]
pub fn riscv_sbi_timer_init_local() {
    assert_eq!(CurrentIrqArch::is_irq_enabled(), false);

    if unsafe { INTERVAL_CNT } == 0 {
        // todo: 将来正式实现时，需要除以HZ
        let new = riscv_time_base_freq() / 3;
        if new == 0 {
            panic!("riscv_sbi_timer_init: failed to get timebase-frequency");
        }
        unsafe {
            INTERVAL_CNT = new;
        }
    }

    let mut guard = SBI_TIMER_INIT_BMP.lock();
    // 如果已经初始化过了，直接返回。或者cpu id不存在
    if guard
        .get(smp_get_processor_id().data() as usize)
        .unwrap_or(true)
    {
        return;
    }

    irq_manager()
        .request_irq(
            RiscVSbiTimer::TIMER_IRQ,
            "riscv_clocksource".to_string(),
            &RiscvSbiTimerHandler,
            IrqHandleFlags::IRQF_SHARED | IrqHandleFlags::IRQF_PERCPU,
            Some(DeviceId::new(Some("riscv sbi timer"), None).unwrap()),
        )
        .expect("Apic timer init failed");

    // 设置第一次中断
    sbi_rt::set_timer(CurrentTimeArch::get_cycles() as u64);

    RiscVSbiTimer::enable();
    guard
        .set(smp_get_processor_id().data() as usize, true)
        .unwrap();
}

#[inline(never)]
pub fn riscv_sbi_timer_irq_desc_init() {
    let desc = irq_desc_manager().lookup(RiscVSbiTimer::TIMER_IRQ).unwrap();

    desc.modify_status(IrqLineStatus::IRQ_LEVEL, IrqLineStatus::empty());
    desc.set_handler(&RiscvSbiTimerIrqFlowHandler);
}

#[derive(Debug)]
struct RiscvSbiTimerHandler;

impl IrqHandler for RiscvSbiTimerHandler {
    fn handle(
        &self,
        _irq: IrqNumber,
        _static_data: Option<&dyn IrqHandlerData>,
        _dynamic_data: Option<Arc<dyn IrqHandlerData>>,
    ) -> Result<IrqReturn, SystemError> {
        // empty (只是为了让编译通过，不会被调用到。真正的处理函数在 RiscvSbiTimerIrqFlowHandler 中)
        Ok(IrqReturn::NotHandled)
    }
}

#[derive(Debug)]
struct RiscvSbiTimerIrqFlowHandler;

impl IrqFlowHandler for RiscvSbiTimerIrqFlowHandler {
    fn handle(&self, _irq_desc: &Arc<IrqDesc>, trap_frame: &mut TrapFrame) {
        RiscVSbiTimer::handle_irq(trap_frame).unwrap();
        fence(Ordering::SeqCst)
    }
}
