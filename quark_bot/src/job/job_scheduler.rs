use crate::dao::dao::Dao;
use crate::job::handler::{job_active_daos, job_daos_results, job_token_ai_fees, job_token_list};
use crate::panora::handler::Panora;
use anyhow::Result;
use teloxide::Bot;
use tokio_cron_scheduler::JobScheduler;

pub async fn schedule_jobs(panora: Panora, bot: Bot, dao: Dao) -> Result<()> {
    let scheduler = JobScheduler::new()
        .await
        .expect("Failed to create job scheduler");

    let job_token_list = job_token_list(panora.clone());
    let job_token_ai_fees = job_token_ai_fees(panora.clone());
    let job_dao_results = job_daos_results(panora.clone(), bot.clone(), dao.clone());
    let job_active_daos = job_active_daos(dao.clone(), bot.clone());

    scheduler
        .add(job_token_list)
        .await
        .expect("Failed to add job to scheduler");
    scheduler
        .add(job_token_ai_fees)
        .await
        .expect("Failed to add job to scheduler");
    scheduler
        .add(job_dao_results)
        .await
        .expect("Failed to add DAO results job to scheduler");
    scheduler
        .add(job_active_daos)
        .await
        .expect("Failed to add DAO active job to scheduler");

    scheduler
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start scheduler: {}", e))?;

    Ok(())
}
