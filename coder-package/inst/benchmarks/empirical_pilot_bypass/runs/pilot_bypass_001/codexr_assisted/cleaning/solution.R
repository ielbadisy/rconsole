patients <- read.csv("patients.csv", stringsAsFactors = FALSE)

required_cols <- c("bmi", "group")
missing_cols <- setdiff(required_cols, names(patients))
if (length(missing_cols) > 0) {
  stop("Missing required columns: ", paste(missing_cols, collapse = ", "))
}

clean <- patients[!is.na(patients$bmi) & patients$bmi >= 10 & patients$bmi <= 80, ]

answer <- data.frame(
  metric = c("clean_n", "mean_bmi", "treated_n"),
  value = c(nrow(clean), mean(clean$bmi), sum(clean$group == "treated", na.rm = TRUE))
)

write.csv(answer, "answer.csv", row.names = FALSE)
