use stratum_common::roles_logic_sv2::parsers_sv2::{
    AnyMessage, CommonMessages, JobDeclaration, Mining, TemplateDistribution,
};

pub fn into_static(m: AnyMessage<'_>) -> AnyMessage<'static> {
    match m {
        AnyMessage::Mining(m) => AnyMessage::Mining(into_static_mining(m)),
        AnyMessage::Common(m) => AnyMessage::Common(into_static_common(m)),
        AnyMessage::JobDeclaration(m) => AnyMessage::JobDeclaration(into_static_job_declaration(m)),
        AnyMessage::TemplateDistribution(m) => {
            AnyMessage::TemplateDistribution(into_static_template_distribution(m))
        }
    }
}

fn into_static_common(m: CommonMessages<'_>) -> CommonMessages<'static> {
    match m {
        CommonMessages::ChannelEndpointChanged(m) => {
            CommonMessages::ChannelEndpointChanged(m.into_static())
        }
        CommonMessages::SetupConnection(m) => CommonMessages::SetupConnection(m.into_static()),
        CommonMessages::SetupConnectionError(m) => {
            CommonMessages::SetupConnectionError(m.into_static())
        }
        CommonMessages::SetupConnectionSuccess(m) => {
            CommonMessages::SetupConnectionSuccess(m.into_static())
        }
        CommonMessages::Reconnect(m) => CommonMessages::Reconnect(m.into_static()),
    }
}

fn into_static_mining(m: Mining<'_>) -> Mining<'static> {
    match m {
        Mining::CloseChannel(m) => Mining::CloseChannel(m.into_static()),
        Mining::NewExtendedMiningJob(m) => Mining::NewExtendedMiningJob(m.into_static()),
        Mining::NewMiningJob(m) => Mining::NewMiningJob(m.into_static()),
        Mining::OpenExtendedMiningChannel(m) => Mining::OpenExtendedMiningChannel(m.into_static()),
        Mining::OpenExtendedMiningChannelSuccess(m) => {
            Mining::OpenExtendedMiningChannelSuccess(m.into_static())
        }
        Mining::OpenMiningChannelError(m) => Mining::OpenMiningChannelError(m.into_static()),
        Mining::OpenStandardMiningChannel(m) => Mining::OpenStandardMiningChannel(m.into_static()),
        Mining::OpenStandardMiningChannelSuccess(m) => {
            Mining::OpenStandardMiningChannelSuccess(m.into_static())
        }
        Mining::SetCustomMiningJob(m) => Mining::SetCustomMiningJob(m.into_static()),
        Mining::SetCustomMiningJobError(m) => Mining::SetCustomMiningJobError(m.into_static()),
        Mining::SetCustomMiningJobSuccess(m) => Mining::SetCustomMiningJobSuccess(m),
        Mining::SetExtranoncePrefix(m) => Mining::SetExtranoncePrefix(m.into_static()),
        Mining::SetGroupChannel(m) => Mining::SetGroupChannel(m.into_static()),
        Mining::SetNewPrevHash(m) => Mining::SetNewPrevHash(m.into_static()),
        Mining::SetTarget(m) => Mining::SetTarget(m.into_static()),
        Mining::SubmitSharesError(m) => Mining::SubmitSharesError(m.into_static()),
        Mining::SubmitSharesExtended(m) => Mining::SubmitSharesExtended(m.into_static()),
        Mining::SubmitSharesStandard(m) => Mining::SubmitSharesStandard(m),
        Mining::SubmitSharesSuccess(m) => Mining::SubmitSharesSuccess(m),
        Mining::UpdateChannel(m) => Mining::UpdateChannel(m.into_static()),
        Mining::UpdateChannelError(m) => Mining::UpdateChannelError(m.into_static()),
    }
}

fn into_static_job_declaration(m: JobDeclaration<'_>) -> JobDeclaration<'static> {
    match m {
        JobDeclaration::AllocateMiningJobToken(m) => {
            JobDeclaration::AllocateMiningJobToken(m.into_static())
        }
        JobDeclaration::AllocateMiningJobTokenSuccess(m) => {
            JobDeclaration::AllocateMiningJobTokenSuccess(m.into_static())
        }
        JobDeclaration::DeclareMiningJob(m) => JobDeclaration::DeclareMiningJob(m.into_static()),
        JobDeclaration::DeclareMiningJobError(m) => {
            JobDeclaration::DeclareMiningJobError(m.into_static())
        }
        JobDeclaration::DeclareMiningJobSuccess(m) => {
            JobDeclaration::DeclareMiningJobSuccess(m.into_static())
        }
        JobDeclaration::ProvideMissingTransactions(m) => {
            JobDeclaration::ProvideMissingTransactions(m.into_static())
        }
        JobDeclaration::ProvideMissingTransactionsSuccess(m) => {
            JobDeclaration::ProvideMissingTransactionsSuccess(m.into_static())
        }
        JobDeclaration::PushSolution(m) => JobDeclaration::PushSolution(m.into_static()),
    }
}

fn into_static_template_distribution(m: TemplateDistribution<'_>) -> TemplateDistribution<'static> {
    match m {
        TemplateDistribution::CoinbaseOutputConstraints(m) => {
            TemplateDistribution::CoinbaseOutputConstraints(m.into_static())
        }
        TemplateDistribution::NewTemplate(m) => TemplateDistribution::NewTemplate(m.into_static()),
        TemplateDistribution::RequestTransactionData(m) => {
            TemplateDistribution::RequestTransactionData(m.into_static())
        }
        TemplateDistribution::RequestTransactionDataError(m) => {
            TemplateDistribution::RequestTransactionDataError(m.into_static())
        }
        TemplateDistribution::RequestTransactionDataSuccess(m) => {
            TemplateDistribution::RequestTransactionDataSuccess(m.into_static())
        }
        TemplateDistribution::SetNewPrevHash(m) => {
            TemplateDistribution::SetNewPrevHash(m.into_static())
        }
        TemplateDistribution::SubmitSolution(m) => {
            TemplateDistribution::SubmitSolution(m.into_static())
        }
    }
}
