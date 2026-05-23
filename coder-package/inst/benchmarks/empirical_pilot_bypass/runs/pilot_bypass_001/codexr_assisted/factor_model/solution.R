d <- read.csv("trial.csv", stringsAsFactors = FALSE)

required <- c("outcome", "treatment", "age", "sex")
if (!all(required %in% names(d))) {
  stop("Missing required columns: ", paste(setdiff(required, names(d)), collapse = ", "))
}

d <- d[required]
d$outcome <- as.integer(d$outcome)
d$age <- as.numeric(d$age)
d$treatment <- factor(d$treatment)
d$sex <- factor(d$sex)

valid <- complete.cases(d) &
  d$outcome %in% c(0L, 1L) &
  d$age > 0 &
  d$treatment %in% c("control", "drug")

analysis <- d[valid, , drop = FALSE]
if (!all(c("control", "drug") %in% levels(droplevels(analysis$treatment)))) {
  stop("Both treatment levels control and drug are required")
}

analysis$treatment <- relevel(droplevels(analysis$treatment), ref = "control")
analysis$sex <- droplevels(analysis$sex)

fit <- glm(outcome ~ treatment + age + sex, data = analysis, family = binomial())
coef_name <- grep("^treatmentdrug$", names(coef(fit)), value = TRUE)
if (length(coef_name) != 1L) {
  stop("Could not identify treatment drug coefficient")
}

answer <- data.frame(
  metric = c("odds_ratio_drug", "n_used"),
  value = c(exp(coef(fit)[coef_name]), nobs(fit))
)

write.csv(answer, "answer.csv", row.names = FALSE)
