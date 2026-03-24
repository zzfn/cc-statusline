.PHONY: release patch minor major

# 读取当前版本
CURRENT_VERSION := $(shell grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
MAJOR := $(word 1, $(subst ., ,$(CURRENT_VERSION)))
MINOR := $(word 2, $(subst ., ,$(CURRENT_VERSION)))
PATCH := $(word 3, $(subst ., ,$(CURRENT_VERSION)))

NEXT_PATCH := $(MAJOR).$(MINOR).$(shell echo $$(($(PATCH)+1)))
NEXT_MINOR := $(MAJOR).$(shell echo $$(($(MINOR)+1))).0
NEXT_MAJOR := $(shell echo $$(($(MAJOR)+1))).0.0

release:
	@echo "当前版本: $(CURRENT_VERSION)"
	@echo ""
	@echo "  1) patch → $(NEXT_PATCH)"
	@echo "  2) minor → $(NEXT_MINOR)"
	@echo "  3) major → $(NEXT_MAJOR)"
	@echo ""
	@read -p "选择 [1/2/3]: " choice; \
	case $$choice in \
	  1) $(MAKE) _do_release V=$(NEXT_PATCH) ;; \
	  2) $(MAKE) _do_release V=$(NEXT_MINOR) ;; \
	  3) $(MAKE) _do_release V=$(NEXT_MAJOR) ;; \
	  *) echo "已取消"; exit 1 ;; \
	esac

_do_release:
	@echo ">>> 更新 Cargo.toml 版本为 $(V)"
	@sed -i '' 's/^version = ".*"/version = "$(V)"/' Cargo.toml
	@cargo update --workspace --quiet
	@git add Cargo.toml Cargo.lock
	@git commit -m "chore: bump version to $(V)"
	@git tag v$(V)
	@git push origin main
	@git push origin v$(V)
	@echo ">>> 发布完成，CI 正在构建 v$(V)"
